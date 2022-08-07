use std::collections::HashMap;

use eyre::{eyre, Result, WrapErr};
use regex::Regex;

use KeyPressEvent::{Note, OtherKeyPress, Special, TextEvent};
use LyricEvent::{
    DuetLyric, DuetPhraseEnd, DuetPhraseStart, Lyric, OtherLyricEvent, PhraseEnd, PhraseStart,
    Section,
};
use TempoEvent::{Anchor, Beat, OtherTempoEvent, TimeSignature};
use crate::TimestampedEvent;

macro_rules! read_capture {
    ($captures:expr, $name:expr) => {
        $captures
            .name($name)
            .ok_or_else(|| eyre!("regex does not contain {}", $name))?
            .as_str()
    };
}

macro_rules! parse {
    ($str:expr) => {
        $str.trim().parse().wrap_err(format!("{:?}", $str))
    };
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
    DuetPhraseStart {
        timestamp: u32,
    },
    DuetPhraseEnd {
        timestamp: u32,
    },
    DuetLyric {
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
            | OtherLyricEvent { timestamp, .. }
            | DuetPhraseStart { timestamp, .. }
            | DuetPhraseEnd { timestamp, .. }
            | DuetLyric { timestamp, .. } => *timestamp,
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

#[derive(Debug)]
pub struct Chart {
    properties: HashMap<String, String>,
    lyrics: Vec<LyricEvent>,
    tempo_map: Vec<TempoEvent>,
    key_presses: HashMap<String, Vec<KeyPressEvent>>,
}

impl Chart {
    /// Creates a chart struct by parsing the passed string representation of a .chart file.
    ///
    /// # Arguments
    ///
    /// * `chart_file`: the contents of the .chart file to parse.
    ///
    /// returns: `Result<Chart, Report>`
    ///
    /// # Errors
    ///
    /// Will return `Err` if the string does not represent a valid .chart file.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs;
    /// use std::io::Read;
    /// use regex::Regex;
    /// use duet_charter_lib::chart::Chart;
    ///
    /// let mut file_content = String::new();
    /// fs::File::open("../charts/Adagio - Second Sight [Peddy]/notes.chart")
    ///     .unwrap()
    ///     .read_to_string(&mut file_content)
    ///     .expect("file reading failed");
    ///
    /// let chart: Chart = Chart::new(&file_content).unwrap();
    /// ```
    pub fn new(chart_file: &str) -> Result<Self> {
        // initialise regexes
        let header_regex = Regex::new("\\[(?P<header>[^]]+)]")?;
        let line_regex =
            Regex::new(" {2}(?P<timestamp>\\d+) = (?P<type>\\w+) (?P<content>[^\\n\\r]+)")?;

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
                "SyncTrack" => Self::decode_tempo_map(&line_regex, &mut tempo_map, section)?,
                "Events" => Self::decode_lyrics(&line_regex, &mut lyrics, section)?,
                &_ => Self::decode_key_presses(&line_regex, &mut key_presses, section, &header)?,
            }
        }
        Ok(Self {
            properties,
            lyrics,
            tempo_map,
            key_presses,
        })
    }

    fn decode_properties(properties: &mut HashMap<String, String>, section: &str) -> Result<()> {
        Regex::new(" {2}(?P<property>[^ =]+) = (?P<content>[^\\n\\r]+)")?
            .captures_iter(section)
            .try_for_each(|captures| {
                let property = read_capture!(captures, "property").to_owned();
                let value = read_capture!(captures, "content").to_owned();
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
                let timestamp = parse!(read_capture!(captures, "timestamp"))?;

                match read_capture!(captures, "type") {
                    "A" => {
                        let song_microseconds = parse!(read_capture!(captures, "content"))?;
                        Ok(Anchor {
                            timestamp,
                            song_microseconds,
                        })
                    }
                    "B" => {
                        let milli_bpm = parse!(read_capture!(captures, "content"))?;
                        Ok(Beat {
                            timestamp,
                            milli_bpm,
                        })
                    }
                    "TS" => {
                        let mut args = read_capture!(captures, "content").split(' ');
                        let pre_numerator = args.next().ok_or_else(|| {
                            eyre!("No numerator found in {}", captures["content"].to_string())
                        })?;
                        let numerator: u32 = parse!(pre_numerator)?;
                        let denominator =
                            2_u32.pow(args.next().map_or(2, |x| parse!(x).unwrap_or(2)));
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
                let timestamp = parse!(read_capture!(captures, "timestamp"))?;
                let code = read_capture!(captures, "type").to_string();
                let content = read_capture!(captures, "content").replace('"', "");
                let (content_type, text) = content.split_once(' ').unwrap_or((&*content, ""));
                let text = text.to_string();
                let result = match (code.as_str(), content_type) {
                    ("E", "section") => Section { timestamp, text },
                    ("E", "phrase_start") => PhraseStart { timestamp },
                    ("E", "lyric") => Lyric { timestamp, text },
                    ("E", "phrase_end") => PhraseEnd { timestamp },
                    ("E", "duet_phrase_start") => DuetPhraseStart { timestamp },
                    ("E", "duet_lyric") => DuetLyric { timestamp, text },
                    ("E", "duet_phrase_end") => DuetPhraseEnd { timestamp },
                    _ => OtherLyricEvent {
                        code,
                        timestamp,
                        content,
                    },
                };
                Ok(result)
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
                let timestamp = parse!(read_capture!(captures, "timestamp"))?;
                let content = read_capture!(captures, "content").to_string();
                match read_capture!(captures, "type") {
                    "N" => {
                        let (key_str, duration_str) = content
                            .split_once(' ')
                            .ok_or_else(|| eyre!("No duration found"))?;

                        let key = parse!(key_str)?;
                        let duration = parse!(duration_str)?;
                        Ok(Note {
                            timestamp,
                            duration,
                            key,
                        })
                    }
                    "S" => {
                        let (type_str, duration_str) = content
                            .split_once(' ')
                            .ok_or_else(|| eyre!("No duration found"))?;
                        let special_type = parse!(type_str)?;
                        let duration = parse!(duration_str)?;
                        Ok(Special {
                            timestamp,
                            duration,
                            special_type,
                        })
                    }
                    "E" => Ok(TextEvent { timestamp, content }),
                    other => Ok(OtherKeyPress {
                        code: other.to_string(),
                        timestamp,
                        content,
                    }),
                }
            })
            .collect::<Result<Vec<_>>>()?;
        key_presses.insert(header.replace('[', "").replace(']', ""), new_notes);
        Ok(())
    }

    #[must_use]
    pub const fn get_properties(&self) -> &HashMap<String, String> {
        &self.properties
    }

    #[must_use]
    pub const fn get_lyrics(&self) -> &Vec<LyricEvent> {
        &self.lyrics
    }

    #[must_use]
    pub const fn get_tempo_map(&self) -> &Vec<TempoEvent> {
        &self.tempo_map
    }

    #[must_use]
    pub const fn get_key_presses(&self) -> &HashMap<String, Vec<KeyPressEvent>> {
        &self.key_presses
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Read;

    use eyre::WrapErr;

    use super::*;

    #[test]
    fn load_test() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("../charts/")?.collect();
        for folder in dir {
            let entry = folder?;
            load_test_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        Ok(())
    }

    fn load_test_helper(folder: &fs::DirEntry) -> Result<()> {
        let mut path = folder.path();
        path.push("notes");
        path.set_extension("chart");
        let mut file = fs::File::open(&path)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;
        Chart::new(&file_content)?;
        Ok(())
    }
}
