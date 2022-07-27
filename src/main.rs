use std::collections::HashMap;
use gloo::file::callbacks::{FileReader, read_as_text};
use gloo::file::File;
use regex::Regex;
use web_sys::{console, Event, HtmlInputElement};
use yew::prelude::*;

use crate::LyricEvent::{Lyric, PhraseEnd, PhraseStart, Section};

enum Msg {
    Files(Vec<File>),
    Loaded(String, String),
}

struct Model {
    readers: HashMap<String, FileReader>,
    chart: Option<Chart>,
}

struct Chart {
    lyric_events: Vec<LyricEvent>,
}

#[derive(Debug)]
enum LyricEvent {
    PhraseStart(u32),
    PhraseEnd(u32),
    Lyric(u32, String),
    Section(u32, String),
}

impl Chart {
    fn from(chart: String) -> Result<Self, String> {
        console::log_1(&"running Chart::from".into());
        let regex = Regex::new(" {2}(?P<timestamp>\\d+) = E \"(?P<type>[^ \"]+)( (?P<content>[^\"]+))?\"").unwrap();
        let mut lyrics = vec![];
        for captures in regex.captures_iter(&chart) {
            console::log_1(&format!("{:?}", captures).into());
            lyrics.push(match &captures["type"] {
                "section" => Section(captures["timestamp"].parse().expect("parsing error"), captures["content"].to_owned()),
                "lyric" => Lyric(captures["timestamp"].parse().expect("parsing error"), captures["content"].to_owned()),
                "phrase_end" => PhraseEnd(captures["timestamp"].parse().expect("parsing error")),
                "phrase_start" => PhraseStart(captures["timestamp"].parse().expect("parsing error")),
                x => panic!("unrecognised lyric event type {}", x)
            });
        }
        Ok(Self { lyric_events: lyrics })
    }
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
                    <p>{ format!("{:?}", chart.lyric_events)} </p>
                }
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}