use std::fmt::{Display, Formatter};
use std::ops::Add;

use color_eyre::eyre::{eyre, Result};

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
pub struct PhraseVec {
    data: Vec<Phrase>,
}

impl PhraseVec {
    pub fn new(lyrics: &[LyricEvent]) -> Result<Self> {
        let data = lyrics
            .iter()
            .filter(|elem| {
                !matches!(
                    elem,
                    LyricEvent::Section { .. } | LyricEvent::Default { .. }
                )
            })
            .collect::<Vec<_>>()
            .split_inclusive(|elem| matches!(elem, LyricEvent::PhraseStart { .. }))
            .filter(|semi_phrase| semi_phrase.len() > 1)
            .map(|mut semi_phrase| {
                if let LyricEvent::PhraseStart { .. } = semi_phrase
                    .first()
                    .ok_or_else(|| eyre!("Empty semi-phrase {:?}", semi_phrase))?
                {
                    semi_phrase = semi_phrase
                        .split_first()
                        .ok_or_else(|| eyre!("Semi-phrase split failed {:?}", semi_phrase))?
                        .1;
                }
                let start_timestamp = semi_phrase
                    .first()
                    .ok_or_else(|| eyre!("Empty semi-phrase {:?}", semi_phrase))?
                    .get_timestamp();
                let end_timestamp = match semi_phrase
                    .last()
                    .ok_or_else(|| eyre!("Empty semi-phrase {:?}", semi_phrase))?
                {
                    LyricEvent::PhraseStart { timestamp } => timestamp - 1,
                    LyricEvent::PhraseEnd { timestamp } => *timestamp,
                    LyricEvent::Lyric { timestamp, .. } => timestamp + 1,
                    _ => unreachable!(),
                };
                let lyrics: Vec<PhraseLyric> = semi_phrase
                    .iter()
                    .filter_map(|elem| match elem {
                        LyricEvent::Lyric { timestamp, text } => Some(PhraseLyric {
                            timestamp: *timestamp,
                            text: text.clone(),
                        }),
                        _ => None,
                    })
                    .collect();
                Ok(Phrase {
                    start_timestamp,
                    end_timestamp,
                    lyrics,
                })
            })
            .collect::<Result<Vec<Phrase>>>()?;
        Ok(Self { data })
    }

    pub const fn get_phrases(&self) -> &Vec<Phrase> {
        &self.data
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Read;

    use color_eyre::eyre::WrapErr;
    use indicatif::ProgressBar;

    use crate::chart::Chart;

    use super::*;

    #[test]
    fn phrase_loading() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("./charts/")?.collect();
        let bar = ProgressBar::new(dir.len() as u64);
        for folder in dir {
            bar.inc(1);
            let entry = folder?;
            phrase_loading_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        bar.finish();
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
        PhraseVec::new(chart.get_lyrics())?;
        Ok(())
    }

    #[test]
    fn phrase_to_string() -> Result<()> {
        let dir: Vec<_> = fs::read_dir("./charts/")?.collect();
        let bar = ProgressBar::new(dir.len() as u64);
        for folder in dir {
            bar.inc(1);
            let entry = folder?;
            phrase_to_string_helper(&entry).wrap_err(format!(
                "Error occurred for chart file {}",
                &entry.file_name().to_str().unwrap_or("filename failure")
            ))?;
        }
        bar.finish();
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
        let phrases = PhraseVec::new(chart.get_lyrics())?;
        let string = phrases
            .get_phrases()
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<String>();
        assert_eq!(
            string.is_empty(),
            chart.get_lyrics().is_empty()
                | chart
                    .get_lyrics()
                    .iter()
                    .all(|x| matches!(x, LyricEvent::Section { .. } | LyricEvent::Default { .. }))
        );
        Ok(())
    }
}
