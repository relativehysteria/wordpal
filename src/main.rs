#![windows_subsystem = "windows"]

use std::process::exit;
use rfd::{MessageDialog, MessageLevel, FileDialog};
use iced::{
    button,
    Button,
    Element,
    Column,
    Row,
    Text,
    Sandbox,
    Settings,
    HorizontalAlignment,
    VerticalAlignment,
    Length,
    Align,
};
use wordpal::db::*;
use wordpal::locale::*;

/// A wrapper around MessageDialog with MessageLevel::Error
fn error(message: &str) {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title(ERROR_WINDOW_TITLE)
            .set_description(message)
            .show();
}

fn main() {
    if App::run(Settings::default()).is_err() {
        error(GENERIC_RUNTIME_ERR_MESSAGE);
        exit(0);
    };
}


#[derive(Clone, Copy, Debug)]
pub enum Message {
    CorrectPressed,
    IncorrectPressed,
    WordPressed,
}

struct App {
    database:         Database,
    current_entry:    Option<(Entry, usize)>,
    word:             String,
    tr_word:          String,
    tr_word_hidden:   bool,
    word_button:      button::State,
    correct_button:   button::State,
    incorrect_button: button::State,
}

impl Sandbox for App {
    type Message = Message;

    fn new() -> Self {
        // Ask for a database file and attempt to open it
        let db = FileDialog::new().pick_file().unwrap_or_else(|| {
            exit(0)
        });
        let mut db = Database::open(db).unwrap_or_else(|err| {
            error(&format!("{}\n\n({})", FAILED_DB_INIT_MESSAGE, err));
            exit(0);
        });

        // Initiate the words so that the ui can show them immediately
        // without any further action
        let entry       = db.random_entry();
        let mut word    = String::new();
        let mut tr_word = String::new();

        if let Some((entry, _)) = &entry {
            word    = entry.word.clone();
            tr_word = entry.tr_word.clone();
        }

        Self {
            word,
            tr_word,
            tr_word_hidden:   true,
            database:         db,
            current_entry:    entry,
            correct_button:   button::State::default(),
            incorrect_button: button::State::default(),
            word_button:      button::State::default(),
        }
    }

    fn title(&self) -> String {
        String::from(ROOT_WINDOW_TITLE)
    }

    fn update(&mut self, message: Message) {
        // If the user clicks on the untranslated word, the translated word
        // is shown/hidden.
        // If they click on either of the correct/incorrect buttons,
        // the entry is timed out.
        match message {
            Message::WordPressed => {
                self.tr_word_hidden = !self.tr_word_hidden;
                return;
            }
            Message::CorrectPressed => {
                if let Some((_, index)) = self.current_entry {
                    self.database.update_timeout(index, true);
                }
                self.tr_word_hidden = true;
            },
            Message::IncorrectPressed => {
                if let Some((_, index)) = self.current_entry {
                    self.database.update_timeout(index, false);
                }
                self.tr_word_hidden = true;
            },
        }

        // Write the database to the file system.
        // XXX: Doing this on every update is slow and can get extreme if done
        //      with larger databases - this should be optimized somehow.
        if let Err(err) = self.database.write_db() {
            error(&format!("{}\n\n({})", FAILED_DB_WRITE_MESSAGE, err));
        }

        // Change the word to the new entry, or set them both to "" if there are
        // no more usable entries.
        self.current_entry = self.database.random_entry();
        if let Some((entry, _)) = &self.current_entry {
            self.word    = entry.word.clone();
            self.tr_word = entry.tr_word.clone();
        } else {
            self.word    = "".to_string();
            self.tr_word = "".to_string();
        }
    }

    fn view(&mut self) -> Element<Message> {
        // Dynamically calculate the font sizes of the words
        let word_size    = 80. / (self.word.len() as f32 / 40.).max(1.);
        let tr_word_size = 50. / (self.tr_word.len() as f32 / 50.).max(1.);

        // Create all the widgets and return.
        // This is how we want the window to look:
        // +---------------+
        // |  -----------  | -> self.word_button
        // |  -----------  | -> self.tr_word (if it isn't hidden)
        // |  ----- -----  | -> self.correct_button | self.incorrect_button
        // +---------------+

        let correct_button = Button::new(&mut self.correct_button,
                                         Text::new(""))
            .on_press(Message::CorrectPressed)
            .min_width(50)
            .min_height(30)
            .width(Length::Fill)
            .style(style::Button::Correct);

        let incorrect_button = Button::new(&mut self.incorrect_button,
                                           Text::new(""))
            .on_press(Message::IncorrectPressed)
            .min_width(50)
            .min_height(30)
            .width(Length::Fill)
            .style(style::Button::Incorrect);

        let word  = Text::new(&self.word)
            .size(word_size as u16)
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center);

        let word_button = Button::new(&mut self.word_button, word)
            .on_press(Message::WordPressed)
            .height(Length::Fill)
            .style(style::Button::Invisible);

        let tr_word = Text::new(&self.tr_word)
            .size(tr_word_size as u16)
            .color(if self.tr_word_hidden {[0.,0.,0.,0.]} else {[0.,0.,0.,1.]})
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Center);

        let horizontal_box = Row::new()
            .align_items(Align::Center)
            .height(Length::Fill)
            .padding(10)
            .spacing(50)
            .push(correct_button)
            .push(incorrect_button);

        let mut col = Column::new()
            .align_items(Align::Center)
            .height(Length::Fill)
            .padding(10)
            .spacing(30);

        // If a word is empty, don't show its widget
        if self.word.len() != 0 {
            col = col.push(word_button);
        }
        if self.tr_word.len() != 0 && !self.tr_word_hidden {
            col = col.push(tr_word);
        }

        col.push(horizontal_box).into()
    }
}

mod style {
    use iced::{button, Background, Color};

    pub enum Button {
        Correct,
        Incorrect,
        Invisible,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            match self {
                Button::Correct => {
                    button::Style {
                        border_color: Color::BLACK,
                        border_width: 2.,
                        background: Some(Background::Color([0.,1.,0.].into())),
                        ..button::Style::default()
                    }
                },
                Button::Incorrect => {
                    button::Style {
                        border_color: Color::BLACK,
                        border_width: 2.,
                        background: Some(Background::Color([1.,0.,0.,].into())),
                        ..button::Style::default()
                    }
                },
                Button::Invisible => {
                    button::Style {
                        border_color: Color::TRANSPARENT,
                        background: Some(Background::Color(Color::TRANSPARENT)),
                        text_color: Color::BLACK,
                        ..button::Style::default()
                    }
                },
            }
        }
    }
}
