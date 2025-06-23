use std::{env, fs, ops::Range, path::PathBuf};

use slp_parser::{Game, Notes, buttons_mask};

enum Note<'a> {
    Text(&'a str),
    // TODO: document how images are compressed, either snoop for symbols in rwing or ask
    // aitch...
    Image(&'a [u8]),
}

trait NotesExt {
    fn add(&mut self, start: i32, len: Option<i32>, note: Note);
    fn add_note(&mut self, frame: i32, note: Note) {
        self.add(frame, None, note);
    }
    fn add_range(&mut self, frames: Range<i32>, note: Note) {
        self.add(frames.start, Some(frames.end - frames.start), note)
    }
}

impl NotesExt for Notes {
    fn add(&mut self, start: i32, len: Option<i32>, note: Note) {
        use Note::*;
        match note {
            Text(s) => {
                let data_idx = self.data.len();
                self.data.push_str(s);
                self.start_frames.push(start);
                self.frame_lengths.push(len.unwrap_or_default());
                self.data_idx.push(data_idx as i32);
            }
            Image(i) => {
                let data_idx = self.image_compressed_data.len();
                self.image_compressed_data.extend_from_slice(i);
                self.image_start_frames.push(start);
                self.image_frame_lengths.push(len.unwrap_or_default());
                self.image_data_offsets.push(data_idx as i32);
            }
        }
    }
}

fn get_presses(game: &Game) -> Vec<usize> {
    // TODO: obviously this check is a brittle hack that just for me. should be easy
    // to provide a list of names and/or slippi connect codes to filter for... or
    // just collect everyone's presses
    let port = game
        .info
        .names
        .map(|arr| String::from_utf8_lossy(&arr).to_string())
        .iter()
        .position(|s| s.starts_with("pineapple"))
        .expect("didn't see your user in this game");

    game.frames[port]
        .as_ref()
        .expect("no frames for your port")
        .iter()
        .enumerate()
        // TODO: use user-specified key, or some key combination to trigger instead of just
        // hardcoding dpad down...
        .filter(|(_, frame)| frame.buttons_mask & buttons_mask::D_PAD_DOWN != 0)
        .map(|(i, _)| i)
        .collect::<Vec<usize>>()
}

fn main() -> Result<(), ()> {
    let path = env::args().nth(1).expect("pass a slippi (or slpz) replay");
    let bytes = fs::read(&path).expect("failed to read replay");
    let res =
        slp_parser::parse_file(&bytes).or_else(|_| slp_parser::parse_file_slpz(&bytes));
    let Ok((game, mut notes)) = res else {
        println!("{:?}", res.err());
        return Err(());
    };

    let mut added = 0;
    'outer: for f in get_presses(&game) {
        let f = f as i32; // why'd aitch make these signed?

        // in an attempt to be idempotent, don't add notes if the start or end frames
        // collide with existing ones
        for (&start, &len) in notes.start_frames.iter().zip(&notes.frame_lengths) {
            if start == f || start + len == f {
                continue 'outer;
            }
        }

        // TODO: use `generate_interactions` to do smarter clip cutoff for start/end
        // instead of a set length, or at least allow the length to be
        // customized. shep probably already wrote this logic so check his impl first
        let clip_len = 300; // hardcoded to 5s for now
        let message = r#"This clip was added because you pressed D-pad down.
To configure this behavior, change your improover settings [here](https://example.com)"#;
        notes.add_range((f - clip_len).max(0)..f, Note::Text(message));
        added += 1;
    }

    println!("Added {added} notes to your replay");
    slp_parser::write_notes_to_game(&PathBuf::from(path), &notes).unwrap();
    Ok(())
}
