import { readFile } from 'node:fs/promises';

const root = new URL('../', import.meta.url);
const creatures = JSON.parse(await readFile(new URL('data/creatures.json', root), 'utf8'));
const packs = JSON.parse(await readFile(new URL('data/packs.json', root), 'utf8'));
const coverage = JSON.parse(await readFile(new URL('data/catalog-coverage.json', root), 'utf8'));
const errors = [];
const check = (condition, message) => { if (!condition) errors.push(message); };
const nonempty = value => typeof value === 'string' && value.trim().length > 0;
const stringList = value => Array.isArray(value) && value.every(nonempty);
const https = value => {
  try { return new URL(value).protocol === 'https:'; }
  catch { return false; }
};
const games = new Set(['ase', 'asa']);
const art = new Set(['theropod', 'sauropod', 'flyer', 'trike', 'aquatic', 'mammal']);
const methods = new Set(['knockout', 'passive', 'special', 'egg', 'breeding', 'unavailable', 'unknown']);
const kinds = new Set(['base', 'variant', 'boss']);
const packKeys = new Set();

check(Array.isArray(packs), 'data/packs.json должен содержать массив');
check(Array.isArray(creatures), 'data/creatures.json должен содержать массив');
if (!Array.isArray(packs) || !Array.isArray(creatures)) throw new Error(errors.join('\n'));

for (const [i, pack] of packs.entries()) {
  const where = `Пакет ${i + 1}`;
  check(pack && typeof pack === 'object', `${where}: нужен объект`);
  if (!pack || typeof pack !== 'object') continue;
  for (const field of ['id', 'name', 'version', 'description']) check(nonempty(pack[field]), `${where}: пустое поле ${field}`);
  check(games.has(pack.game), `${where}: неизвестная игра`);
  check(https(pack.source), `${where}: нужен HTTPS-источник`);
  check(typeof pack.available === 'boolean', `${where}: available должен быть boolean`);
  const key = `${pack.game}:${pack.id}`;
  check(!packKeys.has(key), `${where}: повтор пакета ${key}`);
  packKeys.add(key);
}

for (const required of ['ase:vanilla', 'asa:vanilla', 'ase:ark-additions', 'asa:paleo-ark', 'ase:primal-fear', 'asa:primal-chaos']) {
  check(packKeys.has(required), `Не найден пакет ${required}`);
}

const ids = new Set();
const byId = new Map();
const emptyCounts = () => ({ cards: 0, official: 0, base: 0, variant: 0, boss: 0, mods: 0, partial: 0, own_guides: 0 });
const counts = { ase: emptyCounts(), asa: emptyCounts() };
for (const [i, creature] of creatures.entries()) {
  const where = `Существо ${creature?.slug ?? i + 1}`;
  check(creature && typeof creature === 'object', `${where}: нужен объект`);
  if (!creature || typeof creature !== 'object') continue;
  for (const field of ['slug', 'pack_id', 'pack_version', 'revision', 'name', 'name_en', 'group', 'diet', 'role', 'description']) {
    check(nonempty(creature[field]), `${where}: пустое поле ${field}`);
  }
  check(/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(creature.slug ?? ''), `${where}: slug должен быть стабильным ASCII-идентификатором`);
  check(art.has(creature.art), `${where}: неизвестный силуэт ${creature.art}`);
  check(kinds.has(creature.kind), `${where}: нужен kind=base, variant или boss`);
  check(stringList(creature.aliases), `${where}: aliases должен быть массивом непустых строк`);
  check(stringList(creature.maps) && creature.maps.length > 0, `${where}: нужны карты`);
  const guide = creature.guide;
  check(guide && typeof guide === 'object', `${where}: нет инструкции`);
  if (guide && typeof guide === 'object') {
    check(methods.has(guide.method), `${where}: неизвестный способ приручения`);
    for (const field of ['food', 'equipment', 'steps']) check(stringList(guide[field]), `${where}: ${field} должен быть массивом непустых строк`);
    if (guide.partial === false) {
      check(guide.method !== 'unknown', `${where}: неизвестный способ не может иметь завершённую инструкцию`);
      check(Array.isArray(guide.steps) && guide.steps.length >= 3, `${where}: завершённая инструкция должна содержать не менее трёх шагов`);
      check(Array.isArray(guide.equipment) && guide.equipment.length > 0, `${where}: в завершённой инструкции нет подготовки`);
      if (['knockout', 'passive', 'egg', 'breeding'].includes(guide.method)) {
        check(Array.isArray(guide.food) && guide.food.length > 0, `${where}: в завершённой инструкции нет корма`);
      }
    }
    if (guide.method === 'unavailable') check(guide.food?.length === 0, `${where}: неприручаемая форма не должна получать выдуманный корм`);
    check(stringList(guide.notes), `${where}: notes должен быть массивом строк`);
    check(https(guide.source), `${where}: нужен HTTPS-источник инструкции`);
    check(typeof guide.partial === 'boolean', `${where}: partial должен быть boolean`);
    check(/^\d{4}-\d{2}-\d{2}$/.test(guide.checked_at ?? ''), `${where}: нужна дата проверки YYYY-MM-DD`);
  }
  check(Array.isArray(creature.games) && creature.games.length > 0, `${where}: не указаны игры`);
  if (!Array.isArray(creature.games)) continue;
  check(new Set(creature.games).size === creature.games.length, `${where}: игра указана дважды`);
  for (const game of creature.games) {
    check(games.has(game), `${where}: неизвестная игра ${game}`);
    const key = `${game}:${creature.pack_id}`;
    const pack = packs.find(p => `${p.game}:${p.id}` === key);
    check(Boolean(pack), `${where}: отсутствует пакет ${key}`);
    check(pack?.available === true, `${where}: карточка опубликована в недоступном пакете ${key}`);
    check(pack?.version === creature.pack_version, `${where}: редакция пакета отличается от справочной записи`);
    const id = `${game}:${creature.pack_id}:${creature.slug}`;
    check(!ids.has(id), `Повтор идентификатора ${id}`);
    ids.add(id);
    byId.set(id, creature);
    if (counts[game]) {
      counts[game].cards += 1;
      if (creature.pack_id === 'vanilla') {
        counts[game].official += 1;
        if (kinds.has(creature.kind)) counts[game][creature.kind] += 1;
      } else counts[game].mods += 1;
      counts[game][guide?.partial ? 'partial' : 'own_guides'] += 1;
    }
  }
}

// Protect the original identifiers: existing local collections depend on them.
const originalBase = ['ankylosaurus', 'argentavis', 'baryonyx', 'basilosaurus', 'brontosaurus', 'carbonemys', 'carnotaurus', 'castoroides', 'dimorphodon', 'direwolf', 'dodo', 'doedicurus', 'gigantopithecus', 'ichthyosaurus', 'mammoth', 'megalodon', 'mesopithecus', 'moschops', 'otter', 'pachy', 'parasaur', 'pteranodon', 'pulmonoscorpius', 'quetzal', 'raptor', 'rex', 'sabertooth', 'sarco', 'spino', 'stegosaurus', 'therizinosaur', 'triceratops'];
for (const game of games) {
  for (const slug of originalBase) check(ids.has(`${game}:vanilla:${slug}`), `Удалён исходный ID ${game}:vanilla:${slug}`);
  check(counts[game].base >= (game === 'ase' ? 180 : 200), `${game}: сокращён каталог основных видов/форм`);
  check(counts[game].official >= 300, `${game}: сокращена инвентаризация официального контента`);
}
for (const id of ['ase:ark-additions:brachiosaurus', 'ase:ark-additions:deinosuchus', 'ase:ark-additions:deinotherium', 'asa:paleo-ark:giganotosaurus-evo', 'asa:paleo-ark:tyrannosaurus-evo']) check(ids.has(id), `Удалён исходный ID ${id}`);
check(counts.ase.mods + counts.asa.mods >= 5, 'Нужно не менее пяти модовых вариантов');

// These are source-audited availability boundaries for this catalog revision,
// not a prediction of future ASA releases.
for (const slug of ['astrodelphis', 'maewing', 'noglin', 'shadowmane', 'tek-stryder', 'voidwyrm', 'amargasaurus', 'dinopithecus', 'sinomacrops', 'andrewsarchus', 'fenrir', 'fjordhawk', 'tropeognathus']) {
  check(ids.has(`ase:vanilla:${slug}`), `Не найден официальный ASE DLC-вид ${slug}`);
  check(!ids.has(`asa:vanilla:${slug}`), `ASA ${slug}: наличие требует отдельной проверки перед добавлением`);
}
for (const slug of ['phoenix', 'rock-elemental', 'basilisk', 'karkinos', 'reaper-king', 'rock-drake', 'gacha', 'managarmr', 'snow-owl', 'velonasaur', 'astrocetus', 'bloodstalker', 'ferox', 'magmasaur', 'megachelon', 'deinonychus']) {
  for (const game of games) check(ids.has(`${game}:vanilla:${slug}`), `Не найден DLC-вид ${game}:${slug}`);
}
for (const slug of ['chrysaora', 'dakosaurus', 'homarus', 'tridacna', 'water-wyvern', 'abyssal-rex', 'cymathoa', 'fractalis', 'pygocentrus', 'vulcanithys']) {
  check(ids.has(`ase:vanilla:${slug}`), `Не найден ASE Aquatica-вид ${slug}`);
  check(!ids.has(`asa:vanilla:${slug}`), `Aquatica ${slug} не переносится в ASA автоматически`);
}
check(!ids.has('ase:vanilla:tek-giganotosaurus'), 'ASE Tek Giganotosaurus не спавнится в обычной игре');
check(ids.has('asa:vanilla:tek-giganotosaurus'), 'Не найден ASA Tek Giganotosaurus');
for (const id of ids) check(!/^(asa:vanilla:r-|(?:ase|asa):vanilla:r-dire-bear$)/.test(id), `Неподтверждённый/unused вариант ${id}`);
check(byId.get('ase:vanilla:araneo')?.guide.method === 'passive', 'У ASE Araneo должен сохраняться пассивный способ');
check(byId.get('asa:vanilla:araneo')?.guide.method === 'knockout', 'ASA Araneo после TLC оглушается');
check(byId.get('ase:vanilla:giganotosaurus')?.guide.method === 'knockout', 'ASE Giga оглушается обычным способом');
check(byId.get('asa:vanilla:giganotosaurus')?.guide.method === 'special', 'ASA Giga после 97.0 использует Apex Challenge');
check(byId.get('ase:vanilla:aberrant-megalania')?.maps.includes('Fjordur'), 'ASE Aberrant Megalania: нужен контекст Fjordur');
check(!byId.get('ase:vanilla:aberrant-megalania')?.maps.includes('Aberration'), 'ASE Aberrant Megalania не спавнится на Aberration');

check(coverage.schema_version === 1, 'Неизвестная версия схемы покрытия');
check(coverage.complete === false, 'Не заявляйте полное покрытие без отдельного аудита всех границ');
check(nonempty(coverage.scope), 'Не описаны границы каталога');
check(stringList(coverage.boundaries) && coverage.boundaries.length > 0, 'Не описаны ограничения достоверности');
check(Array.isArray(coverage.sources) && coverage.sources.length >= 14, 'Нужны исходные списки карт/DLC');
for (const source of coverage.sources ?? []) check(nonempty(source.subject) && https(source.url), 'Некорректный источник инвентаризации');
check(Array.isArray(coverage.excluded_or_pending) && coverage.excluded_or_pending.length > 0, 'Не перечислены непокрытые разделы');
for (const entry of coverage.excluded_or_pending ?? []) check(nonempty(entry.scope) && nonempty(entry.reason) && https(entry.source), 'Некорректная граница покрытия');
for (const game of games) {
  const inventory = coverage.inventory?.[game];
  check(Array.isArray(inventory), `${game}: нет измеримой инвентаризации`);
  if (!Array.isArray(inventory)) continue;
  const measured = new Set();
  for (const item of inventory) {
    check(!measured.has(item.id), `Повтор ID в инвентаризации ${item.id}`);
    measured.add(item.id);
    const creature = byId.get(item.id);
    check(Boolean(creature) && item.id.startsWith(`${game}:`), `Лишний ID в инвентаризации ${item.id}`);
    check(item.name === creature?.name_en && item.partial === creature?.guide.partial, `Инвентаризация устарела для ${item.id}`);
    check(item.kind === (creature?.pack_id === 'vanilla' ? creature?.kind : 'mod'), `Тип инвентаризации устарел для ${item.id}`);
    check(https(item.list_source), `${item.id}: нет источника списка`);
  }
  for (const id of ids) if (id.startsWith(`${game}:`)) check(measured.has(id), `ID отсутствует в инвентаризации ${id}`);
  for (const [field, value] of Object.entries(counts[game])) check(coverage.counts?.[game]?.[field] === value, `${game}: счётчик покрытия ${field} не совпадает с данными`);
}
for (const pack of packs) check(pack.version === coverage.catalog_version, `${pack.game}:${pack.id}: версия покрытия устарела`);

if (errors.length) {
  console.error(errors.map(error => `- ${error}`).join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Каталог проверен: ${creatures.length} исходных записей, ${ids.size} карточек.`);
  for (const game of games) console.log(`${game.toUpperCase()}: ${counts[game].base} основных видов/форм, ${counts[game].variant} вариантов, ${counts[game].boss} боссов, ${counts[game].mods} модовых.`);
  console.log(`Неполные инструкции: ASE ${counts.ase.partial}, ASA ${counts.asa.partial}. Источники и даты проверены по структуре; игровые сценарии этот валидатор не запускает.`);
}
