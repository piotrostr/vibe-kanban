use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

const MAX_LINES: usize = 1000;

pub struct LogsState {
    pub lines: VecDeque<String>,
    pub scroll_offset: usize,
    pub log_path: PathBuf,
    last_position: u64,
}

impl LogsState {
    pub fn new() -> Self {
        let log_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibe")
            .join("vibe.log");

        Self {
            lines: VecDeque::new(),
            scroll_offset: 0,
            log_path,
            last_position: 0,
        }
    }

    pub fn load_logs(&mut self) {
        if let Ok(file) = File::open(&self.log_path) {
            let reader = BufReader::new(file);
            self.lines.clear();

            for line in reader.lines().flatten() {
                self.lines.push_back(line);
                if self.lines.len() > MAX_LINES {
                    self.lines.pop_front();
                }
            }

            // Scroll to bottom by default
            self.scroll_offset = self.lines.len().saturating_sub(1);
        }
    }

    pub fn refresh(&mut self) {
        // Read new lines from log file
        if let Ok(mut file) = File::open(&self.log_path) {
            // Seek to last known position
            if file.seek(SeekFrom::Start(self.last_position)).is_ok() {
                let reader = BufReader::new(&mut file);

                for line in reader.lines().flatten() {
                    self.lines.push_back(line);
                    if self.lines.len() > MAX_LINES {
                        self.lines.pop_front();
                    }
                }

                // Update position
                if let Ok(pos) = file.stream_position() {
                    self.last_position = pos;
                }
            }
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_offset < self.lines.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    pub fn visible_lines(&self, height: usize) -> impl Iterator<Item = &String> {
        let start = self.scroll_offset.saturating_sub(height / 2);
        let end = (start + height).min(self.lines.len());
        self.lines.iter().skip(start).take(end - start)
    }
}

impl Default for LogsState {
    fn default() -> Self {
        Self::new()
    }
}
