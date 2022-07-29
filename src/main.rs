use std::collections::HashMap;

use color_eyre::eyre::{ErrReport, Result};
use gloo::file::callbacks::{read_as_text, FileReader};
use gloo::file::File;
use web_sys::{console, HtmlInputElement};
use yew::prelude::*;

use chart::Chart;
use chart::KeyPressEvent::{Note, Special, TextEvent};
use chart::LyricEvent::{Lyric, PhraseEnd, PhraseStart, Section};
use chart::TempoEvent::{Anchor, Beat, TimeSignature};

mod chart;

enum Msg {
    Files(Vec<File>),
    Loaded(String, String),
}

struct Main {
    readers: HashMap<String, FileReader>,
    chart: Option<Chart>,
    error: Option<ErrReport>,
}

impl Component for Main {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        console::log_1(&"Hello using web-sys".into());
        Self {
            readers: HashMap::default(),
            chart: None,
            error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Files(files) => {
                for file in files {
                    let file_name = file.name();
                    let task = {
                        let file_name = file_name.clone();
                        let link = ctx.link().clone();
                        read_as_text(&file, move |res| {
                            link.send_message(Msg::Loaded(
                                file_name,
                                res.unwrap_or_else(|e| e.to_string()),
                            ));
                        })
                    };
                    self.readers.insert(file_name, task);
                }
                false
            }
            Msg::Loaded(file_name, data) => {
                self.readers.remove(&file_name);
                match Chart::from(&data) {
                    Ok(chart) => {
                        self.chart = Some(chart);
                        self.error = None;
                    }
                    Err(err) => {
                        self.chart = None;
                        self.error = Some(err);
                    }
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let _link = ctx.link();
        html! {
            <div>
                <input type="file" accept=".chart" onchange={ctx.link().callback(move |e: Event| {
                            let mut result = Vec::new();
                            let input: HtmlInputElement = e.target_unchecked_into();

                            if let Some(files) = input.files() {
                                let files = js_sys::try_iter(&files)
                                    .unwrap()
                                    .unwrap()
                                    .map(|v| web_sys::File::from(v.unwrap()))
                                    .map(File::from);
                                result.extend(files);
                            }
                            Msg::Files(result)
                        })}
                    />

                if let Some(chart) = &self.chart {
                    <div>
                        <section id = "properties">
                        <h1>{ "Properties:" }</h1>
                        <ul>
                        { for chart.get_properties().iter().map(|(name, content)| html!{ <li> { format!("{}: {}", name, content) } </li> }) }
                        </ul>
                        </section>
                        <section id = "synctrack">
                        <h1>{ "SyncTrack:" }</h1>
                        <ul>
                        { for chart.get_sync_track().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                        </section>
                        <section id = "lyrics">
                        <h1>{ "Lyrics:" }</h1>
                        <ul>
                        { for chart.get_lyrics().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                        </section>
                        <section id = "phrases">
                        <h1>{ "Phrases:" }</h1>
                        <ul>
                        { for chart.get_phrases().unwrap().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                        </section>
                        <section id = "notes">
                        <h1>{ "Notes:" }</h1>
                        <ol>
                        { for chart.get_key_presses().iter().map(|(difficulty, notes)| html!{ <li> { format!("{:?}", difficulty) } <ul> {for notes.iter().map(|event|html!{ <li> { format!("{:?}", event) } </li> })} </ul> </li> }) }
                        </ol>
                        </section>
                    </div>
                }
            </div>
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    yew::start_app::<Main>();
    Ok(())
}
