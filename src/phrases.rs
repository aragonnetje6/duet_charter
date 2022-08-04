use std::fmt::{Display, Formatter};
use std::ops::Add;

use eyre::Result;

use crate::chart::LyricEvent;
use crate::chart::TimestampedEvent;

#[derive(Debug)]
pub struct PhraseLyric {
    timestamp: u32,
    text: String,
}

#[derive(Debug)]
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
pub struct LyricPhrases {
    main: Vec<Phrase>,
    duet: Vec<Phrase>,
}

impl LyricPhrases {
    pub fn new(lyrics_events: &[LyricEvent]) -> Self {
        let duet_only = lyrics_events
            .iter()
            .filter_map(|event| match event {
                LyricEvent::DuetPhraseStart { timestamp } => Some(LyricEvent::PhraseStart {
                    timestamp: *timestamp,
                }),
                LyricEvent::DuetPhraseEnd { timestamp } => Some(LyricEvent::PhraseEnd {
                    timestamp: *timestamp,
                }),
                LyricEvent::DuetLyric { timestamp, text } => Some(LyricEvent::Lyric {
                    timestamp: *timestamp,
                    text: text.clone(),
                }),
                LyricEvent::PhraseStart { .. }
                | LyricEvent::PhraseEnd { .. }
                | LyricEvent::Lyric { .. }
                | LyricEvent::Section { .. }
                | LyricEvent::OtherLyricEvent { .. } => None,
            })
            .collect::<Vec<LyricEvent>>();
        let main = Self::parse_phrases_from(lyrics_events);
        let duet = Self::parse_phrases_from(&duet_only);
        Self { main, duet }
    }

    fn parse_phrases_from(lyric_events: &[LyricEvent]) -> Vec<Phrase> {
        let timestamps: Vec<u32> = lyric_events
            .iter()
            .filter_map(|x| match x {
                LyricEvent::PhraseStart { timestamp } => Some(*timestamp),
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
                        LyricEvent::Lyric { timestamp, text } => Some(PhraseLyric {
                            timestamp: *timestamp,
                            text: text.clone(),
                        }),
                        _ => None,
                    })
                    .collect();
                let maybe_timestamp = lyric_events.iter().find(|x| {
                    (high.is_none() || high.unwrap_or(&0) >= &x.get_timestamp())
                        && x.get_timestamp() > *low
                        && matches!(x, LyricEvent::PhraseEnd { .. })
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

    pub const fn get_main_phrases(&self) -> &Vec<Phrase> {
        &self.main
    }

    pub const fn get_duet_phrases(&self) -> &Vec<Phrase> {
        &self.duet
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Read;

    use color_eyre::eyre::WrapErr;

    use crate::chart::Chart;

    use super::*;

    #[test]
    fn phrase_loading() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("./charts/")?.collect();
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
        let chart = Chart::from(&file_content)?;
        assert_eq!(
            LyricPhrases::new(chart.get_lyrics()).main.len(),
            chart
                .get_lyrics()
                .iter()
                .filter(|x| matches!(x, LyricEvent::PhraseStart { .. }))
                .count()
        );
        Ok(())
    }

    #[test]
    fn phrase_to_string() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("./charts/")?.collect();
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
        let chart = Chart::from(&file_content)?;
        let phrases = LyricPhrases::new(chart.get_lyrics());
        let string = phrases
            .get_main_phrases()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<String>();
        assert_eq!(
            string.is_empty(),
            !chart.get_lyrics().iter().any(|x| !matches!(
                x,
                LyricEvent::Section { .. } | LyricEvent::OtherLyricEvent { .. }
            ))
        );
        Ok(())
    }
}
