use std::collections::HashMap;

use color_eyre::eyre::{eyre, Result};
use regex::Regex;

use KeyPressEvent::OtherKeyPress;
use LyricEvent::OtherLyricEvent;
use TempoEvent::OtherTempoEvent;

use crate::{
    Anchor, Beat, Lyric, Note, PhraseEnd, PhraseStart, Section, Special, TextEvent, TimeSignature,
};

pub trait TimestampedEvent {
    fn get_timestamp(&self) -> u32;
}

#[derive(Debug)]
pub enum LyricEvent {
    PhraseStart {
        timestamp: u32,
    },
    PhraseEnd {
        timestamp: u32,
    },
    Lyric {
        timestamp: u32,
        text: String,
    },
    Section {
        timestamp: u32,
        text: String,
    },
    OtherLyricEvent {
        code: String,
        timestamp: u32,
        content: String,
    },
}

impl TimestampedEvent for LyricEvent {
    fn get_timestamp(&self) -> u32 {
        match self {
            PhraseStart { timestamp, .. }
            | PhraseEnd { timestamp, .. }
            | Lyric { timestamp, .. }
            | Section { timestamp, .. }
            | OtherLyricEvent { timestamp, .. } => *timestamp,
        }
    }
}

#[derive(Debug)]
pub enum KeyPressEvent {
    Note {
        timestamp: u32,
        duration: u32,
        key: u32,
    },
    Special {
        timestamp: u32,
        special_type: u32,
        duration: u32,
    },
    TextEvent {
        timestamp: u32,
        content: String,
    },
    OtherKeyPress {
        code: String,
        timestamp: u32,
        content: String,
    },
}

impl TimestampedEvent for KeyPressEvent {
    fn get_timestamp(&self) -> u32 {
        match self {
            Note { timestamp, .. }
            | Special { timestamp, .. }
            | TextEvent { timestamp, .. }
            | OtherKeyPress { timestamp, .. } => *timestamp,
        }
    }
}

#[derive(Debug)]
pub enum TempoEvent {
    Beat {
        timestamp: u32,
        milli_bpm: u64,
    },
    TimeSignature {
        timestamp: u32,
        time_signature: (u32, u32),
    },
    Anchor {
        timestamp: u32,
        song_microseconds: u64,
    },
    OtherTempoEvent {
        code: String,
        timestamp: u32,
        content: String,
    },
}

impl TimestampedEvent for TempoEvent {
    fn get_timestamp(&self) -> u32 {
        match self {
            Beat { timestamp, .. }
            | TimeSignature { timestamp, .. }
            | Anchor { timestamp, .. }
            | OtherTempoEvent { timestamp, .. } => *timestamp,
        }
    }
}

pub struct Chart {
    properties: HashMap<String, String>,
    lyrics: Vec<LyricEvent>,
    tempo_map: Vec<TempoEvent>,
    key_presses: HashMap<String, Vec<KeyPressEvent>>,
}

impl Chart {
    pub fn from(chart_file: &str) -> Result<Self> {
        // initialise regexes
        let header_regex = Regex::new("\\[(?P<header>[^]]+)]")?;
        let universal_regex =
            Regex::new(" {2}(?P<timestamp>\\d+) = (?P<type>\\w+) (?P<content>[^\\n\\r]+)")?;
        let lyrics_regex =
            Regex::new(" {2}(?P<timestamp>\\d+) = E \"(?P<type>[^ \"]+)( (?P<text>[^\"]+))?\"")?;
        let notes_regex =
            Regex::new(" {2}(?P<timestamp>\\d+) = (?P<type>[NSE]) (?P<key>.) (?P<duration>\\d)?")?;

        // declare output variables
        let mut properties = HashMap::new();
        let mut lyrics = vec![];
        let mut tempo_map = vec![];
        let mut key_presses = HashMap::new();

        // decode file
        for section in chart_file.split('}') {
            let header = match header_regex.find(section) {
                None => continue,
                Some(x) => x.as_str().replace('[', "").replace(']', ""),
            };
            match header.as_str() {
                "Song" => Self::decode_properties(&mut properties, section)?,
                "SyncTrack" => Self::decode_tempo_map(&universal_regex, &mut tempo_map, section)?,
                "Events" => Self::decode_lyrics(&lyrics_regex, &mut lyrics, section)?,
                &_ => Self::decode_key_presses(&notes_regex, &mut key_presses, section, &header)?,
            }
        }
        Ok(Self {
            properties,
            lyrics,
            tempo_map,
            key_presses,
        })
    }

    fn decode_properties(
        properties: &mut HashMap<String, String>,
        section: &str,
    ) -> Result<()> {
        Regex::new(" {2}(?P<property>[^ =]+) = (?P<content>[^\\n\\r]+)")?
            .captures_iter(section).try_for_each(|captures| {
            let property = captures
                .name("property")
                .ok_or_else(|| eyre!("no property name found"))?
                .as_str()
                .to_owned();
            let value = captures
                .name("content")
                .ok_or_else(|| eyre!("no property value found"))?
                .as_str()
                .to_owned();
            properties.insert(property, value);
            Ok(())
        })
    }

    fn decode_tempo_map(
        regex: &Regex,
        tempo_map: &mut Vec<TempoEvent>,
        section: &str,
    ) -> Result<()> {
        let new_tempo_map: Vec<TempoEvent> = regex
            .captures_iter(section)
            .map(|captures| -> Result<TempoEvent> {
                let timestamp = captures["timestamp"].parse()?;

                match &captures["type"] {
                    "A" => {
                        let song_microseconds = captures["content"].parse()?;
                        Ok(Anchor {
                            timestamp,
                            song_microseconds,
                        })
                    }
                    "B" => {
                        let milli_bpm = captures["content"].parse()?;
                        Ok(Beat {
                            timestamp,
                            milli_bpm,
                        })
                    }
                    "TS" => {
                        let mut args = captures["content"].split(' ');
                        let numerator = args
                            .next()
                            .ok_or_else(|| {
                                eyre!("No numerator found in {}", captures["content"].to_string())
                            })?
                            .parse()?;
                        let denominator =
                            2_u32.pow(args.next().map_or(2, |x| x.parse().unwrap_or(2)));
                        let time_signature = (numerator, denominator);
                        Ok(TimeSignature {
                            timestamp,
                            time_signature,
                        })
                    }
                    other => {
                        let code = other.to_string();
                        let content = captures
                            .name("content")
                            .map_or_else(|| "", |x| x.as_str())
                            .to_string();
                        Ok(OtherTempoEvent {
                            code,
                            timestamp,
                            content,
                        })
                    }
                }
            })
            .collect::<Result<_>>()?;
        tempo_map.extend(new_tempo_map);
        Ok(())
    }

    fn decode_lyrics(regex: &Regex, lyrics: &mut Vec<LyricEvent>, section: &str) -> Result<()> {
        let new_lyrics = regex
            .captures_iter(section)
            .map(|captures| -> Result<LyricEvent> {
                let timestamp = captures["timestamp"].parse()?;
                match &captures["type"] {
                    "section" => Ok(Section {
                        timestamp,
                        text: captures["text"].to_owned(),
                    }),
                    "lyric" => Ok(Lyric {
                        timestamp,
                        text: captures["text"].to_owned(),
                    }),
                    "phrase_end" => Ok(PhraseEnd { timestamp }),
                    "phrase_start" => Ok(PhraseStart { timestamp }),
                    "Default" => Ok(OtherLyricEvent {
                        code: "".to_string(),
                        timestamp,
                        content: "".to_string(),
                    }),
                    err => Err(eyre!("unrecognised lyric event type {}", err)),
                }
            })
            .collect::<Result<Vec<LyricEvent>>>()?;
        lyrics.extend(new_lyrics);
        Ok(())
    }

    fn decode_key_presses(
        regex: &Regex,
        key_presses: &mut HashMap<String, Vec<KeyPressEvent>>,
        section: &str,
        header: &str,
    ) -> Result<()> {
        let new_notes: Vec<KeyPressEvent> = regex
            .captures_iter(section)
            .map(|captures| -> Result<KeyPressEvent> {
                let timestamp = captures["timestamp"].parse()?;
                let duration = captures["duration"].parse()?;
                match &captures["type"] {
                    "N" => {
                        let key = captures["key"].parse()?;
                        Ok(Note {
                            timestamp,
                            duration,
                            key,
                        })
                    }
                    "S" => {
                        let special_type = captures["key"].parse()?;
                        Ok(Special {
                            timestamp,
                            duration,
                            special_type,
                        })
                    }
                    "E" => Ok(TextEvent {
                        timestamp,
                        content: captures["key"].to_owned(),
                    }),
                    x => Err(eyre!("unrecognised keypress type {}", x)),
                }
            })
            .collect::<Result<Vec<_>>>()?;
        key_presses.insert(header.replace('[', "").replace(']', ""), new_notes);
        Ok(())
    }

    pub const fn get_properties(&self) -> &HashMap<String, String> {
        &self.properties
    }

    pub const fn get_lyrics(&self) -> &Vec<LyricEvent> {
        &self.lyrics
    }

    pub const fn get_tempo_map(&self) -> &Vec<TempoEvent> {
        &self.tempo_map
    }

    pub const fn get_key_presses(&self) -> &HashMap<String, Vec<KeyPressEvent>> {
        &self.key_presses
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Read;

    use color_eyre::eyre::WrapErr;
    use indicatif::ProgressBar;

    use super::*;

    #[test]
    fn load_test() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("./charts/")?.collect();
        let bar = ProgressBar::new(dir.len() as u64);
        for folder in dir {
            bar.inc(1);
            let entry = folder?;
            load_test_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        bar.finish();
        Ok(())
    }

    fn load_test_helper(folder: &fs::DirEntry) -> Result<()> {
        let mut path = folder.path();
        path.push("notes");
        path.set_extension("chart");
        let mut file = fs::File::open(&path)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;
        Chart::from(&file_content)?;
        Ok(())
    }
}
