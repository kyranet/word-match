use std::fmt;

use crate::confusables::Confusable;

#[napi]
#[derive(PartialEq)]
pub enum Boundary {
	/// The start of a new word.
	Start,
	/// The contents of a word.
	Word,
	/// The end of a word.
	End,
	/// A single character that is both the start and the end of a word.
	Mixed,
	/// A non-word character, which can be skipped, such as punctuation, spaces,
	/// or newlines.
	NoContent,
}

impl Boundary {
	pub(crate) fn is_word(&self) -> bool {
		match self {
			Boundary::Start | Boundary::Word | Boundary::End | Boundary::Mixed => true,
			Boundary::NoContent => false,
		}
	}
}

#[napi]
pub struct Sentence {
	// TODO: checked is useful, but it could be even more useful by having a vector of spans which split up as needed.
	// For example, if "drowned" is marked in "Steve drowned in lava", upon marking, the regions would become from
	// [0..=21] to [0..=6] ("Steve ") and [13..=21] (" in lava"), where [7..=12] ("drowned") is removed. However, an
	// improved algorithm would include the spaces (trimming sideways) so the ranges become [0..=5] ("Steve") and
	// [14..=21] ("in lava"), which reduces the amount of checks. However, `checked` must stay untrimmed or else when
	// the words are censored, it includes the spaces ("Steve*********in lava") which is suboptimal compared to the
	// opposite ("Steve ******* in lava").
	/// A vector of booleans that represent whether the character at the same
	/// index has been checked by a `Word`.
	pub checked: Vec<bool>,
	/// A vector of `Boundary` that represent the boundaries of the words in the
	/// sentence.
	pub boundaries: Vec<Boundary>,
	/// A vector of characters that represent the sanitized contents of the
	/// sentence.
	pub(crate) contents: Vec<char>,
	/// A vector of indexes that represent the start and end of each word in the
	/// sentence.
	pub(crate) indexes: Vec<(usize, usize)>,
}

#[napi]
impl Sentence {
	#[napi(constructor)]
	pub fn new(sentence: String) -> Self {
		let sentence = sentence.replace_confusables().to_lowercase();
		let mut checked: Vec<bool> = Vec::with_capacity(sentence.len());
		let mut boundaries: Vec<Boundary> = Vec::with_capacity(sentence.len());
		let mut contents: Vec<char> = Vec::with_capacity(sentence.len());
		let mut indexes: Vec<(usize, usize)> = Vec::with_capacity(sentence.len());

		let mut chars = sentence.chars().peekable();
		// TODO: Rewrite this loop to a nested loop for improved efficiency, code
		// readability, and reduce code duplication.
		while let Some(c) = chars.next() {
			let mut boundary = if c.is_whitespace() || c.is_control() {
				Boundary::NoContent
			} else if let Some(c) = boundaries.last() {
				if c.is_word() {
					Boundary::Word
				} else {
					Boundary::Start
				}
			} else {
				Boundary::Start
			};

			if boundary == Boundary::Start {
				let start = contents.len();
				indexes.push((start, start));

				// If the next character is a whitespace or control character, the boundary is
				// mixed as this character is both the start and the end of a word.
				if let Some(c) = chars.peek() {
					if c.is_whitespace() || c.is_control() {
						boundary = Boundary::Mixed;
					}
				} else {
					boundary = Boundary::Mixed;
				}
			} else if boundary == Boundary::Word {
				// If the next character is a whitespace or control character, the boundary is
				// end as this character is the end of a word.
				if let Some(c) = chars.peek() {
					if c.is_whitespace() || c.is_control() {
						boundary = Boundary::End;
					}
				} else {
					boundary = Boundary::End;
				}
			}

			checked.push(false);
			boundaries.push(boundary);
			contents.push(c);
		}

		Self { checked, boundaries, contents, indexes }
	}

	#[napi(getter)]
	pub fn length(&self) -> u32 {
		self.contents.len() as u32
	}

	#[napi(js_name = "toString")]
	pub fn js_to_string(&self) -> String {
		self.to_string()
	}
}

impl fmt::Display for Sentence {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.contents.iter().collect::<String>())
	}
}
