use std::collections::HashMap;

use color_eyre::eyre::{eyre, ErrReport, Result};
use gloo::file::callbacks::{read_as_text, FileReader};
use gloo::file::File;
use web_sys::{console, HtmlInputElement};
use yew::prelude::*;

use chart::Chart;
use chart::KeyPressEvent::{Note, Special, TextEvent};
use chart::LyricEvent::{Lyric, PhraseEnd, PhraseStart, Section};
use chart::TempoEvent::{Anchor, Beat, TimeSignature};

use crate::phrases::PhraseVec;

mod chart;
mod phrases;

enum Msg {
    Files(Result<Vec<File>>),
    Loaded(String, String),
    Parsed(),
}

struct Main {
    readers: HashMap<String, FileReader>,
    chart: Option<Chart>,
    error: Option<ErrReport>,
    phrases: Option<PhraseVec>,
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
            phrases: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link().clone();
        match msg {
            Msg::Files(files) => {
                if let Ok(files_vec) = files {
                    for file in files_vec {
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
                }
                false
            }
            Msg::Loaded(file_name, data) => {
                self.readers.remove(&file_name);
                match Chart::from(&data) {
                    Ok(chart) => {
                        self.chart = Some(chart);
                        self.error = None;
                        link.send_message(Msg::Parsed());
                    }
                    Err(err) => {
                        self.chart = None;
                        self.error = Some(err);
                    }
                };
                true
            }
            Msg::Parsed() => match &self.chart {
                None => false,
                Some(chart) => {
                    match PhraseVec::new(chart.get_lyrics()) {
                        Ok(phrases) => self.phrases = Some(phrases),
                        Err(x) => self.error = Some(x),
                    };
                    true
                }
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        fn helper(input: &HtmlInputElement) -> Result<Vec<File>> {
            if let Some(files) = input.files() {
                js_sys::try_iter(&files)
                    .unwrap_or(None)
                    .ok_or_else(|| eyre!("No file"))?
                    .map(|v| -> Result<web_sys::File> { match v {
                        Ok(x) => {Ok(web_sys::File::from(x))}
                        Err(_) => {Err(eyre!("file loading error"))}
                    }} )
                    .map(|v| { let v2 = v?; Ok(File::from(v2)) })
                    .collect()
            } else {
                Ok(Vec::new())
            }
        }
        let _link = ctx.link();
        html! {
            <>
                <input type="file" accept=".chart" onchange={
                    ctx.link().callback(move |e: Event| Msg::Files(helper(&e.target_unchecked_into())))
                }/>

                if let Some(chart) = &self.chart {
                    <>
                        <section id = "toc">
                            <h1>{ "Table of Contents" }</h1>
                            <ol>
                                <li><a href="#properties">{ "Properties" }</a></li>
                                <li><a href="#tempomap">{ "Tempo map" }</a></li>
                                <li><a href="#lyrics">{ "Lyrics" }</a></li>
                                <li><a href="#notes">{ "Notes" }</a></li>
                                <li><a href="#phrases">{ "Phrases" }</a></li>
                            </ol>
                        </section>
                        <section id = "properties">
                            <h1>{ "Properties:" }</h1>
                            <a href="#toc">{ "^" }</a>
                            <ul>
                                { for chart.get_properties().iter().map(|(name, content)| html!{ <li> { format!("{}: {}", name, content) } </li> }) }
                            </ul>
                        </section>
                            <section id = "tempomap">
                            <h1>{ "Tempo map:" }</h1>
                            <a href="#toc">{ "^" }</a>
                            <ul>
                                { for chart.get_tempo_map().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                            </ul>
                        </section>
                        <section id = "lyrics">
                            <h1>{ "Lyrics:" }</h1>
                            <a href="#toc">{ "^" }</a>
                            <ul>
                                { for chart.get_lyrics().iter().map(|event| html!{ <li> { format!("{:?}", event) } </li> }) }
                            </ul>
                        </section>
                        <section id = "notes">
                            <h1>{ "Notes:" }</h1>
                            <a href="#toc">{ "^" }</a>
                            <ol>
                                { for chart.get_key_presses().iter().map(|(difficulty, notes)| html!{ <li> { format!("{:?}", difficulty) } <ul> {for notes.iter().map(|event|html!{ <li> { format!("{:?}", event) } </li> })} </ul> </li> }) }
                            </ol>
                        </section>
                    </>
                }
                if let Some(phrases) = &self.phrases {
                <section id = "phrases">
                    <h1>{ "Phrases:" }</h1>
                    <a href="#toc">{ "^" }</a>
                    <ul>
                        { for phrases.get_phrases().iter().map(|event| html!{ <li> { format!("{}", event) } </li> }) }
                    </ul>
                </section>
                }
            </>
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    yew::start_app::<Main>();
    Ok(())
}
