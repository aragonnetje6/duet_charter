use std::collections::HashMap;

use gloo::file::callbacks::{read_as_text, FileReader};
use gloo::file::File;
use regex::Regex;
use web_sys::{console, HtmlInputElement};
use yew::prelude::*;

use crate::KeyPressEvent::{Note, Strum};
use crate::LyricEvent::{Lyric, PhraseEnd, PhraseStart, Section};

enum Msg {
    Files(Vec<File>),
    Loaded(String, String),
}

trait ChartEvent {
    fn get_timestamp(&self) -> u32;
}

#[derive(Debug)]
enum LyricEvent {
    PhraseStart { timestamp: u32 },
    PhraseEnd { timestamp: u32 },
    Lyric { timestamp: u32, text: String },
    Section { timestamp: u32, text: String },
}

impl ChartEvent for LyricEvent {
    fn get_timestamp(&self) -> u32 {
        match self {
            PhraseStart { timestamp } => *timestamp,
            PhraseEnd { timestamp } => *timestamp,
            Lyric { timestamp, .. } => *timestamp,
            Section { timestamp, .. } => *timestamp,
        }
    }
}

#[derive(Debug)]
enum KeyPressEvent {
    Note {
        timestamp: u32,
        duration: u32,
        key: u8,
    },
    Strum {
        timestamp: u32,
        duration: u32,
    },
}

impl ChartEvent for KeyPressEvent {
    fn get_timestamp(&self) -> u32 {
        match self {
            Note { timestamp, .. } => *timestamp,
            Strum { timestamp, .. } => *timestamp,
        }
    }
}

struct Chart {
    lyric_events: Vec<LyricEvent>,
    key_presses: Vec<KeyPressEvent>,
}

impl Chart {
    fn from(chart_file: String) -> Result<Self, String> {
        Ok(Self {
            lyric_events: Self::get_lyrics(&chart_file),
            key_presses: Self::get_notes(&chart_file),
        })
    }

    fn get_lyrics(chart_file: &String) -> Vec<LyricEvent> {
        let regex =
            Regex::new(" {2}(?P<timestamp>\\d+) = E \"(?P<type>[^ \"]+)( (?P<content>[^\"]+))?\"")
                .unwrap();
        let mut lyrics = vec![];
        for captures in regex.captures_iter(&chart_file) {
            lyrics.push(match &captures["type"] {
                "section" => Section {
                    timestamp: captures["timestamp"].parse().expect("parsing error"),
                    text: captures["content"].to_owned(),
                },
                "lyric" => Lyric {
                    timestamp: captures["timestamp"].parse().expect("parsing error"),
                    text: captures["content"].to_owned(),
                },
                "phrase_end" => PhraseEnd {
                    timestamp: captures["timestamp"].parse().expect("parsing error"),
                },
                "phrase_start" => PhraseStart {
                    timestamp: captures["timestamp"].parse().expect("parsing error"),
                },
                x => panic!("unrecognised lyric event type {}", x),
            });
        }
        lyrics
    }

    fn get_notes(chart_file: &String) -> Vec<KeyPressEvent> {
        let regex =
            Regex::new(" {2}(?P<timestamp>\\d+) = (?P<type>[NS]) (?P<key>\\d) (?P<duration>\\d)")
                .unwrap();
        let mut notes = vec![];
        for captures in regex.captures_iter(&chart_file) {
            notes.push(match &captures["type"] {
                "N" => Note {
                    timestamp: captures["timestamp"].parse().expect("parsing error"),
                    duration: captures["duration"].parse().expect("parsing error"),
                    key: captures["key"].parse().expect("parsing error"),
                },
                "S" => Strum {
                    timestamp: captures["timestamp"].parse().expect("parsing error"),
                    duration: captures["duration"].parse().expect("parsing error"),
                },
                x => panic!("unrecognised keypress type {}", x),
            });
        }
        notes
    }
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
                self.chart = Some(Chart::from(data).unwrap());
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
                        <p>{ "Lyrics:" }</p>
                        <ul>
                        { for chart.lyric_events.iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                        </ul>
                        <p>{ "Notes:" }</p>
                        <ul>
                        { for chart.key_presses.iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
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
