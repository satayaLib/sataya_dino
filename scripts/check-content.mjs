import { readFile } from 'node:fs/promises';

const root = new URL('../', import.meta.url);
const creatures = JSON.parse(await readFile(new URL('data/creatures.json', root), 'utf8'));
const packs = JSON.parse(await readFile(new URL('data/packs.json', root), 'utf8'));
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
const methods = new Set(['knockout', 'passive', 'special', 'egg']);
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
const counts = { ase: { base: 0, mods: 0, partial: 0 }, asa: { base: 0, mods: 0, partial: 0 } };
for (const [i, creature] of creatures.entries()) {
  const where = `Существо ${creature?.slug ?? i + 1}`;
  check(creature && typeof creature === 'object', `${where}: нужен объект`);
  if (!creature || typeof creature !== 'object') continue;
  for (const field of ['slug', 'pack_id', 'pack_version', 'revision', 'name', 'name_en', 'group', 'diet', 'role', 'description']) {
    check(nonempty(creature[field]), `${where}: пустое поле ${field}`);
  }
  check(/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(creature.slug ?? ''), `${where}: slug должен быть стабильным ASCII-идентификатором`);
  check(art.has(creature.art), `${where}: неизвестный силуэт ${creature.art}`);
  check(stringList(creature.aliases), `${where}: aliases должен быть массивом непустых строк`);
  check(stringList(creature.maps) && creature.maps.length > 0, `${where}: нужны карты`);
  const guide = creature.guide;
  check(guide && typeof guide === 'object', `${where}: нет инструкции`);
  if (guide && typeof guide === 'object') {
    check(methods.has(guide.method), `${where}: неизвестный способ приручения`);
    for (const field of ['food', 'equipment', 'steps']) check(stringList(guide[field]) && guide[field].length > 0, `${where}: нет ${field}`);
    check(Array.isArray(guide.steps) && guide.steps.length >= 3, `${where}: инструкция должна содержать не менее трёх шагов`);
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
    if (counts[game]) {
      counts[game][creature.pack_id === 'vanilla' ? 'base' : 'mods'] += 1;
      if (guide?.partial) counts[game].partial += 1;
    }
  }
}

for (const game of games) check(counts[game].base >= 30, `${game}: нужно не менее 30 базовых записей`);
check(counts.ase.mods + counts.asa.mods >= 5, 'Нужно не менее пяти модовых вариантов');

if (errors.length) {
  console.error(errors.map(error => `- ${error}`).join('\n'));
  process.exitCode = 1;
} else {
  console.log(`Каталог проверен: ${ids.size} карточек; ASE ${counts.ase.base} + ${counts.ase.mods} модовых; ASA ${counts.asa.base} + ${counts.asa.mods} модовых.`);
  console.log(`Неполные инструкции: ASE ${counts.ase.partial}, ASA ${counts.asa.partial}. Источники и даты проверены по структуре; игровые сценарии этот валидатор не запускает.`);
}
