use std::collections::HashMap;

use gloo::file::callbacks::{FileReader, read_as_text};
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

struct Model {
    readers: HashMap<String, FileReader>,
    chart: Option<Chart>,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        console::log_1(&"Hello using web-sys".into());
        Self {
            readers: HashMap::default(),
            chart: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Files(files) => {
                for file in files.into_iter() {
                    let file_name = file.name();
                    let task = {
                        let file_name = file_name.clone();
                        let link = ctx.link().clone();
                        read_as_text(&file, move |res| {
                            link.send_message(Msg::Loaded(
                                file_name,
                                res.unwrap_or_else(|e| e.to_string()),
                            ))
                        })
                    };
                    self.readers.insert(file_name, task);
                }
                true
            }
            Msg::Loaded(file_name, data) => {
                self.readers.remove(&file_name);
                self.chart = Some(Chart::from(data));
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
                        <p>{ "SyncTrack:" }</p>
                        <ul>
                        { for chart.get_sync_track().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                        <p>{ "Lyrics:" }</p>
                        <ul>
                        { for chart.get_lyrics().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                        <p>{ "Notes:" }</p>
                        <ul>
                        { for chart.get_key_presses().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                    </div>
                }
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
