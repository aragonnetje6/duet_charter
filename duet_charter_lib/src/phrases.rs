use std::fmt::{Display, Formatter};
use std::ops::Add;

use crate::chart::LyricEvent;
use crate::chart::LyricEvent::{
    DuetLyric, DuetPhraseEnd, DuetPhraseStart, Lyric, OtherLyricEvent, PhraseEnd, PhraseStart,
    Section,
};
use crate::TimestampedEvent;

#[derive(Debug, Clone)]
pub struct PhraseLyric {
    timestamp: u32,
    text: String,
}

impl TimestampedEvent for PhraseLyric {
    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
}

#[derive(Debug, Clone)]
pub struct Phrase {
    start_timestamp: u32,
    end_timestamp: u32,
    lyrics: Vec<PhraseLyric>,
}

impl Display for Phrase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let line = self
            .lyrics
            .iter()
            .map(|x| x.text.clone())
            .map(|x| {
                let y = x.clone().add(" ");
                x.strip_suffix('-').unwrap_or(y.as_str()).to_string()
            })
            .collect::<String>();
        let clean_line = line.strip_suffix(' ').unwrap_or(line.as_str()).to_string();
        write!(
            f,
            "from {} to {}, phrase: {}",
            self.start_timestamp, self.end_timestamp, clean_line
        )
    }
}

#[derive(Debug)]
pub struct LyricPhraseCollection {
    main_phrases: Vec<Phrase>,
    duet_phrases: Vec<Phrase>,
}

impl LyricPhraseCollection {
    /// Constructor for `LyricPhraseCollection` from a collection of `LyricEvent`s.
    ///
    /// # Arguments
    ///
    /// * `lyrics_events`: the collection of `LyricEvent`s to build the phrases out of.
    ///
    /// returns: `LyricPhraseCollection`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs;
    /// use std::io::Read;
    /// use regex::Regex;
    /// use duet_charter_lib::chart::Chart;
    /// use duet_charter_lib::phrases::LyricPhraseCollection;
    ///
    /// let mut file_content = String::new();
    /// fs::File::open("../charts/Adagio - Second Sight [Peddy]/notes.chart")
    ///     .unwrap()
    ///     .read_to_string(&mut file_content)
    ///     .expect("file reading failed");
    ///
    /// let chart = Chart::new(&file_content).unwrap();
    /// let phrases = LyricPhraseCollection::new(chart.get_lyrics());
    /// ```
    #[must_use]
    pub fn new(lyrics_events: &[LyricEvent]) -> Self {
        let duet_only = lyrics_events
            .iter()
            .filter_map(|event| match event {
                DuetPhraseStart { timestamp } => Some(PhraseStart {
                    timestamp: *timestamp,
                }),
                DuetPhraseEnd { timestamp } => Some(PhraseEnd {
                    timestamp: *timestamp,
                }),
                DuetLyric { timestamp, text } => Some(Lyric {
                    timestamp: *timestamp,
                    text: text.clone(),
                }),
                PhraseStart { .. }
                | PhraseEnd { .. }
                | Lyric { .. }
                | Section { .. }
                | OtherLyricEvent { .. } => None,
            })
            .collect::<Vec<LyricEvent>>();
        let main = Self::parse_phrases_from(lyrics_events);
        let duet = Self::parse_phrases_from(&duet_only);
        Self {
            main_phrases: main,
            duet_phrases: duet,
        }
    }

    fn parse_phrases_from(lyric_events: &[LyricEvent]) -> Vec<Phrase> {
        let timestamps: Vec<u32> = lyric_events
            .iter()
            .filter_map(|x| match x {
                PhraseStart { timestamp } => Some(*timestamp),
                _ => None,
            })
            .collect();
        timestamps
            .iter()
            .enumerate()
            .map(|(i, low)| {
                let high = timestamps.get(i + 1);
                let lyrics: Vec<PhraseLyric> = lyric_events
                    .iter()
                    .filter(|lyric| {
                        (high.is_none() || high.unwrap_or(&0) > &lyric.get_timestamp())
                            && lyric.get_timestamp() >= *low
                    })
                    .filter_map(|lyric| match lyric {
                        Lyric { timestamp, text } => Some(PhraseLyric {
                            timestamp: *timestamp,
                            text: text.clone(),
                        }),
                        _ => None,
                    })
                    .collect();
                let maybe_timestamp = lyric_events.iter().find(|x| {
                    (high.is_none() || high.unwrap_or(&0) >= &x.get_timestamp())
                        && x.get_timestamp() > *low
                        && matches!(x, PhraseEnd { .. })
                });
                let end_timestamp = match maybe_timestamp {
                    Some(x) => x.get_timestamp(),
                    None => match high {
                        Some(y) => *y,
                        None => match lyrics.last() {
                            Some(z) => z.timestamp + 1,
                            None => low + 1,
                        },
                    },
                };
                Phrase {
                    start_timestamp: *low,
                    end_timestamp,
                    lyrics,
                }
            })
            .collect()
    }

    pub fn encode(&self) -> Vec<LyricEvent> {
        let main = self.main_phrases.clone();
        let mut duet = self.duet_phrases.clone();
        let mut result: Vec<LyricEvent> = vec![];
        for main_phrase in main.iter() {
            match duet.first() {
                None => Self::encode_single(main_phrase, &mut result),
                Some(duet_phrase) => {
                    if duet_phrase.start_timestamp > main_phrase.start_timestamp {
                        Self::encode_single(main_phrase, &mut result);
                    } else {
                        todo!()
                    }
                }
            }
        }
        result
    }

    fn encode_single(phrase: &Phrase, result: &mut Vec<LyricEvent>) {
        result.push(PhraseStart {
            timestamp: phrase.start_timestamp,
        });
        for syllable in &phrase.lyrics {
            result.push(Lyric {
                timestamp: syllable.timestamp,
                text: syllable.text.clone(),
            });
        }
        result.push(PhraseEnd {
            timestamp: phrase.end_timestamp,
        });
    }

    #[must_use]
    pub const fn get_main_phrases(&self) -> &Vec<Phrase> {
        &self.main_phrases
    }

    #[must_use]
    pub const fn get_duet_phrases(&self) -> &Vec<Phrase> {
        &self.duet_phrases
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Read;

    use eyre::{Result, WrapErr};

    use crate::chart::Chart;

    use super::*;

    #[test]
    fn phrase_loading() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("../charts/")?.collect();
        for folder in dir {
            let entry = folder?;
            phrase_loading_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        Ok(())
    }

    fn phrase_loading_helper(folder: &fs::DirEntry) -> Result<()> {
        let mut path = folder.path();
        path.push("notes");
        path.set_extension("chart");
        let mut file = fs::File::open(&path)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;
        let chart = Chart::new(&file_content)?;
        assert_eq!(
            LyricPhraseCollection::new(chart.get_lyrics())
                .main_phrases
                .len(),
            chart
                .get_lyrics()
                .iter()
                .filter(|x| matches!(x, PhraseStart { .. }))
                .count()
        );
        Ok(())
    }

    #[test]
    fn phrase_to_string() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("../charts/")?.collect();
        for folder in dir {
            let entry = folder?;
            phrase_to_string_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        Ok(())
    }

    fn phrase_to_string_helper(folder: &fs::DirEntry) -> Result<()> {
        let mut path = folder.path();
        path.push("notes");
        path.set_extension("chart");
        let mut file = fs::File::open(&path)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;
        let chart = Chart::new(&file_content)?;
        let phrases = LyricPhraseCollection::new(chart.get_lyrics());
        let string = phrases
            .get_main_phrases()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<String>();
        assert_eq!(
            string.is_empty(),
            !chart
                .get_lyrics()
                .iter()
                .any(|x| !matches!(x, Section { .. } | OtherLyricEvent { .. }))
        );
        Ok(())
    }
}
