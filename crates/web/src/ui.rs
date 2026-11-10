use wasm_bindgen::JsCast;
use web_sys::{HtmlDialogElement, HtmlSelectElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SelectFieldProps {
    pub value: AttrValue,
    #[prop_or_default]
    pub onchange: Callback<Event>,
    #[prop_or_default]
    pub label: Option<AttrValue>,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or(false)]
    pub required: bool,
    pub children: Children,
}

#[component]
pub fn SelectField(props: &SelectFieldProps) -> Html {
    let node = use_node_ref();
    {
        let node = node.clone();
        let value = props.value.clone();
        use_effect(move || {
            if let Some(select) = node.cast::<HtmlSelectElement>() {
                select.set_value(&value);
            }
            || ()
        });
    }
    html! {<select ref={node} onchange={props.onchange.clone()} aria-label={props.label.clone()} disabled={props.disabled} required={props.required}>{props.children.clone()}</select>}
}

#[derive(Properties, PartialEq)]
pub struct IconProps {
    pub name: AttrValue,
    #[prop_or_default]
    pub class: Classes,
}

#[component]
pub fn Icon(props: &IconProps) -> Html {
    let path = match props.name.as_str() {
        "grid" => "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
        "book" => "M12 6c-3-3-7-3-10-2v15c4-1 7-1 10 2 3-3 6-3 10-2V4c-4-1-7-1-10 2Zm0 0v15",
        "box" => "m3 7 9-5 9 5v10l-9 5-9-5V7Zm0 0 9 5 9-5M12 12v10",
        "plus" => "M12 5v14M5 12h14",
        "search" => "M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z",
        "close" => "m6 6 12 12M6 18 18 6",
        "check" => "m5 12 4 4 10-10",
        "heart" => {
            "M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z"
        }
        "star" => "m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9L12 3Z",
        "photo" => "M4 6h4l2-3h4l2 3h4v15H4V6Zm8 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8Z",
        "download" => "M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5",
        "upload" => "M12 16V4m-5 5 5-5 5 5M4 16v5h16v-5",
        "transfer" => "M3 7h17m-4-4 4 4-4 4M21 17H4m4-4-4 4 4 4",
        "settings" => {
            "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1 1-3Z"
        }
        "chevron" => "m9 5 7 7-7 7",
        "external" => "M14 3h7v7m0-7L11 13M9 3H3v18h18v-6",
        "back" => "M20 12H4m7-7-7 7 7 7",
        "filter" => "M4 5h16M7 12h10M10 19h4",
        "trash" => "M3 6h18M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7M14 10v7",
        "copy" => "M8 8h13v13H8zM16 8V3H3v13h5",
        "compare" => "M5 3v18M12 8v13M19 5v16M2 21h20",
        "globe" => {
            "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18M12 3c5 5 5 13 0 18-5-5-5-13 0-18Z"
        }
        "moon" => "M21 13A9 9 0 0 1 11 3a9 9 0 1 0 10 10Z",
        "menu" => "M4 6h16M4 12h16M4 18h16",
        _ => "M12 8v5m0 4h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
    };
    html! {<svg class={classes!("icon",props.class.clone())} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={path}/></svg>}
}

#[derive(Properties, PartialEq)]
pub struct ArtProps {
    pub kind: AttrValue,
    #[prop_or_default]
    pub class: Classes,
}

#[component]
pub fn DinoArt(props: &ArtProps) -> Html {
    let d = match props.kind.as_str() {
        "sauropod" => {
            "M19 133Q62 130 91 114Q116 90 156 101L178 111Q190 104 192 80L191 35Q188 20 200 18L220 23L230 33L212 39L207 80Q211 111 192 128L187 151L180 174H168L170 149L162 134L132 133L116 151L108 175H96L98 146L91 128Q62 146 19 137Z"
        }
        "flyer" => {
            "M19 41L116 92L134 88L143 62L162 41L176 39L188 49L169 53L157 73L158 93L175 101L267 49L232 102L181 111L162 121L165 142L176 157L166 157L148 140L136 152L125 149L142 129L139 115L118 108L67 96Z"
        }
        "trike" => {
            "M28 124Q56 104 87 102Q112 79 156 90L179 99L199 71L217 78L221 94L247 87L236 111L257 118L269 123L257 136L229 138L212 154L199 166H188L192 144L178 139L166 170H153L155 140L118 136L99 171H87L89 141L69 132L30 134Z"
        }
        "aquatic" => {
            "M21 114L62 98Q112 75 164 94L182 74L180 102L219 96L258 111L273 126L240 133L215 128L191 129L176 153L169 132Q114 140 66 122L24 147L38 122Z"
        }
        "mammal" => {
            "M25 111L62 106L88 90Q121 76 160 93L190 96L200 68L217 63L227 83L249 88L261 101L241 114L225 111L212 136L204 166H192L190 138L170 130L153 163H141L146 130L109 125L93 163H82L85 131L62 123L24 122Z"
        }
        _ => {
            "M13 132Q65 127 108 100Q137 73 172 86L185 79L191 59L208 49L246 52L260 65L258 79L233 83L227 91L248 93L244 102L218 103L207 91L197 106L184 114L190 124L210 128L213 136L199 136L181 130L172 120L161 126L165 142L182 164L176 173L159 170L140 149L130 134L117 139L111 160L94 174L80 174L83 164L97 149L99 126Q60 148 13 137Z"
        }
    };
    html! {<svg class={classes!("dino-art",props.class.clone())} viewBox="0 0 290 190" aria-hidden="true"><path d={d} fill="currentColor"/><path d="M17 179h255" stroke="currentColor" stroke-width=".6" opacity=".2"/></svg>}
}

#[derive(Properties, PartialEq)]
pub struct ModalProps {
    pub title: AttrValue,
    pub onclose: Callback<()>,
    pub children: Children,
    #[prop_or(false)]
    pub wide: bool,
}

#[component]
pub fn Modal(props: &ModalProps) -> Html {
    let node = use_node_ref();
    {
        let node = node.clone();
        use_effect_with((), move |_| {
            if let Some(d) = node.cast::<HtmlDialogElement>() {
                let _ = d.show_modal();
            }
            || ()
        });
    }
    let onclose = {
        let cb = props.onclose.clone();
        Callback::from(move |_: Event| cb.emit(()))
    };
    let close = {
        let cb = props.onclose.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };
    let backdrop = {
        let node = node.clone();
        let cb = props.onclose.clone();
        Callback::from(move |e: MouseEvent| {
            if e.target()
                .and_then(|t| t.dyn_into::<HtmlDialogElement>().ok())
                .is_some()
                && let Some(dialog) = node.cast::<HtmlDialogElement>()
            {
                let r = dialog.get_bounding_client_rect();
                if (e.client_x() as f64) < r.left()
                    || (e.client_x() as f64) > r.right()
                    || (e.client_y() as f64) < r.top()
                    || (e.client_y() as f64) > r.bottom()
                {
                    cb.emit(());
                }
            }
        })
    };
    html! {<dialog ref={node} class={classes!("modal",props.wide.then_some("wide"))} onclose={onclose} onclick={backdrop} aria-label={props.title.clone()}>
        <header class="modal-header"><div><span class="eyebrow">{"ЛИЧНАЯ КОЛЛЕКЦИЯ"}</span><h2>{&props.title}</h2></div><button class="icon-button" onclick={close} aria-label="Закрыть"><Icon name="close"/></button></header>
        {props.children.clone()}
    </dialog>}
}

pub fn method_label(method: &str) -> &str {
    match method {
        "knockout" => "Оглушение",
        "passive" => "Пассивное",
        "special" => "Особый способ",
        "egg" => "Выращивание",
        _ => "Нет данных",
    }
}
pub fn status_label(status: &str) -> &str {
    match status {
        "active" => "Активен",
        "cryopod" => "В криоподе",
        "dead" => "Погиб",
        "transferred" => "Передан",
        _ => status,
    }
}
