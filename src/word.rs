use std::char;

use napi::{Error, Status};

use crate::{
	confusables::Confusable,
	constants::*,
	sentence::{Boundary, Sentence},
	word_part::*,
};

#[napi]
pub struct WordMatch {
	pub(crate) start: usize,
	pub(crate) end: usize,
}

#[napi]
impl WordMatch {
	#[napi(getter, enumerable = true, js_name = "start")]
	pub fn get_start(&self) -> u32 {
		self.start.try_into().unwrap()
	}

	#[napi(getter, enumerable = true, js_name = "end")]
	pub fn get_end(&self) -> u32 {
		self.end.try_into().unwrap()
	}
}

/// The `Word` struct represents a word that can be matched against a
/// `Sentence`.
///
/// It is composed of a series of `WordPart`s, which can be a single character,
/// a group of characters (`[abc]`), a single wildcard (`*`), or an any wildcard
/// (`**`).
///
/// It is intended to be used to match words within a `Sentence` using
/// [`matches`](Word::matches).
#[napi]
#[derive(Clone)]
pub struct Word {
	parts: Vec<WordPart>,
	/// If `true`, the word must match the left boundary of the sentence.
	pub bound_left: bool,
	/// If `true`, the word must match the right boundary of the sentence.
	pub bound_right: bool,
}

#[napi]
impl Word {
	#[napi(constructor)]
	pub fn new(word: String) -> Result<Self, Error> {
		let word = word.replace_confusables().to_lowercase();
		let mut chars = word.chars().peekable();

		let mut parts: Vec<WordPart> = Vec::new();
		while let Some(c) = chars.next() {
			let part = match c {
				ASTERISK => {
					if chars.peek() == Some(&ASTERISK) {
						// Found '**':
						chars.next();
						WordPart::AnyWildcard
					} else {
						// Found '*':
						WordPart::SingleWildcard
					}
				}
				GROUP_START => {
					// Found '['
					let mut group: Vec<char> = Vec::new();
					loop {
						match chars.next() {
							// If ']' is found, it is the end of the Group
							Some(GROUP_END) => {
								break;
							}
							// If '\' is found, it is an escape character, read the next character
							Some(ESCAPE) => {
								if let Some(c) = chars.next() {
									group.push(c);
								}
							}
							// If a character is found, add it to the Group
							Some(c) => {
								if !group.contains(&c) {
									group.push(c);
								}
							}
							// If the end of the word is reached, return an error
							None => {
								return Err(Error::new(Status::GenericFailure, "Unterminated character group"));
							}
						}
					}

					if group.is_empty() {
						// If the group is empty, skip it:
						continue;
					} else if group.len() == 1 {
						// If the group has only one character, add it as a Single character
						WordPart::Single(group[0])
					} else {
						// If the group has more than one character, add it as a Group
						WordPart::Group(group)
					}
				}
				ESCAPE => {
					// Found '\'
					if let Some(c) = chars.next() {
						WordPart::Single(c)
					} else {
						return Err(Error::new(
							Status::GenericFailure,
							"Escape character cannot be at the end of the word",
						));
					}
				}
				_ => WordPart::Single(c),
			};

			parts.push(part);
		}

		if parts.is_empty() {
			return Err(Error::new(Status::GenericFailure, "The word cannot be empty"));
		}

		let bound_left = parts.first().unwrap() != &WordPart::AnyWildcard;
		if !bound_left {
			if parts.len() == 1 {
				return Err(Error::new(Status::GenericFailure, "Wildcards cannot be the only character in the word"));
			}

			parts.remove(0);
		}

		let bound_right = parts.last().unwrap() != &WordPart::AnyWildcard;
		if !bound_right {
			if parts.len() == 1 {
				return Err(Error::new(Status::GenericFailure, "Wildcards cannot be the only character in the word"));
			}

			parts.pop();
		}

		Ok(Word { parts, bound_left, bound_right })
	}

	/// Matches a `Word` against a `Sentence`.
	///
	/// This method will iterate over the `Sentence`'s `spans` and attempt to
	/// match the `Word` against each word in the `Sentence`.
	///
	/// For each match, the `Sentence` will be updated to mark the matched word,
	/// and the method will return `true` if at least one match was found.
	///
	/// @remarks
	///
	/// If the same word is run multiple times against the same `Sentence`
	/// instance, it will still try to match the word against the entire
	/// sentence, even if it has already been matched, returning `false`.
	pub fn matches(&self, sentence: &mut Sentence) -> bool {
		let mut matched = false;
		for (start, end) in sentence.spans.clone().iter() {
			// If the word is longer than the sentence span, skip it:
			if self.parts.len() > (end - start) {
				continue;
			}

			let matches = match (self.bound_left, self.bound_right) {
				(true, true) => self.matches_full(sentence, *start, *end),
				(true, false) => self.matches_prefix(sentence, *start, *end),
				(false, true) => self.matches_suffix(sentence, *start, *end),
				(false, false) => self.matches_infix(sentence, *start, *end),
			};

			if matches {
				matched = true;
			}
		}

		matched
	}

	/// Matches a full word within a sentence, iterating over `Sentence`'s
	/// `spans` finding the `Start` word boundaries and matching until the `End`
	/// word boundary.
	fn matches_full(&self, sentence: &mut Sentence, start: usize, end: usize) -> bool {
		let last_part_index = self.parts.len() - 1;

		let mut matched = false;
		let mut word_index = 0usize;
		let mut previous_character = None;

		// Iterate over a sentence span:
		for i in start..end {
			// Find the start of the word:
			if !sentence.boundaries[i].is_start() {
				continue;
			}

			if !self.parts[word_index].matches(sentence.contents[i], previous_character) {
				continue;
			}

			let match_start = i;
			let mut match_end = usize::MAX;

			// Iterate over a possible word:
			for i in i..end {
				let character = sentence.contents[i];
				let boundary = sentence.boundaries[i];

				// If the boundary is not a word boundary, skip it:
				if boundary == Boundary::NoContent {
					continue;
				}

				// If the current word index matches the character, move to the next character:
				if self.parts[word_index].matches(character, previous_character) {
					previous_character = Some(character);
					if word_index != last_part_index {
						word_index += 1;
					} else if boundary.is_end() {
						match_end = i;
					}

					continue;
				}

				// If it does, and neither does the last character, reset the state:
				if !self.parts[word_index - 1].matches(character, previous_character) {
					word_index = 0;
					previous_character = None;
					break;
				}
			}

			if match_end != usize::MAX {
				matched = true;
				sentence.mark(match_start, match_end);
			}
		}

		matched
	}

	/// Matches a prefix within a sentence, iterating over `Sentence`'s `spans`
	/// finding the `Start` word boundaries and matching until the `End` word
	/// boundary.
	fn matches_prefix(&self, sentence: &mut Sentence, start: usize, end: usize) -> bool {
		let last_part_index = self.parts.len() - 1;

		let mut matched = false;
		let mut word_index = 0usize;
		let mut previous_character = None;

		// Iterate over a sentence span:
		for i in start..end {
			// Find the start of the word:
			if !sentence.boundaries[i].is_start() {
				continue;
			}

			if !self.parts[word_index].matches(sentence.contents[i], previous_character) {
				continue;
			}

			let match_start = i;
			let mut match_end = usize::MAX;

			// Iterate over a possible word:
			for i in i..end {
				let character = sentence.contents[i];
				let boundary = sentence.boundaries[i];

				// If the boundary is not a word boundary, skip it:
				if boundary == Boundary::NoContent {
					continue;
				}

				// If the current word index matches the character, move to the next character:
				if self.parts[word_index].matches(character, previous_character) {
					previous_character = Some(character);
					if word_index != last_part_index {
						word_index += 1;
					} else {
						match_end = i;
					}

					continue;
				}

				// If it does, and neither does the last character, reset the state:
				if !self.parts[word_index - 1].matches(character, previous_character) {
					word_index = 0;
					previous_character = None;
					break;
				}
			}

			if match_end != usize::MAX {
				matched = true;
				sentence.mark(match_start, match_end);
			}
		}

		matched
	}

	/// Matches a suffix within a sentence, iterating over `Sentence`'s `spans`
	/// finding the `End` word boundaries and matching in reverse until the
	/// `End` word boundary.
	fn matches_suffix(&self, sentence: &mut Sentence, start: usize, end: usize) -> bool {
		let last_part_index = self.parts.len() - 1;

		let mut matched = false;
		let mut word_index = last_part_index;
		let mut previous_character = None;

		// Iterate over a sentence span:
		for i in (start..end).rev() {
			// Find the start of the word:
			if !sentence.boundaries[i].is_end() {
				continue;
			}

			if !self.parts[word_index].matches(sentence.contents[i], previous_character) {
				continue;
			}

			let match_start = i;
			let mut match_end = usize::MAX;

			// Iterate over a possible word:
			for i in (start..end).rev() {
				let character = sentence.contents[i];
				let boundary = sentence.boundaries[i];

				// If the boundary is not a word boundary, skip it:
				if boundary == Boundary::NoContent {
					continue;
				}

				// If the current word index matches the character, move to the next character:
				if self.parts[word_index].matches(character, previous_character) {
					previous_character = Some(character);
					if word_index != 0 {
						word_index -= 1;
					} else {
						match_end = i;
					}

					continue;
				}

				// If it does, and neither does the last character, reset the state:
				if !self.parts[word_index + 1].matches(character, previous_character) {
					word_index = last_part_index;
					previous_character = None;
					break;
				}
			}

			if match_end != usize::MAX {
				matched = true;
				sentence.mark(match_start, match_end);
			}
		}

		matched
	}

	/// Matches an infix within a sentence, iterating over `Sentence`'s `spans`
	/// finding the `Start` word boundaries and matching until the `End` word
	/// boundary.
	fn matches_infix(&self, sentence: &mut Sentence, start: usize, end: usize) -> bool {
		let last_part_index = self.parts.len() - 1;

		let mut matched = false;
		let mut word_index = 0usize;
		let mut previous_character = None;

		// Iterate over a sentence span:
		for i in start..end {
			if !self.parts[word_index].matches(sentence.contents[i], previous_character) {
				continue;
			}

			let match_start = i;
			let mut match_end = usize::MAX;

			// Iterate over a possible word:
			for i in i..end {
				let character = sentence.contents[i];
				let boundary = sentence.boundaries[i];

				// If the boundary is not a word boundary, skip it:
				if boundary == Boundary::NoContent {
					continue;
				}

				// If the current word index matches the character, move to the next character:
				if self.parts[word_index].matches(character, previous_character) {
					previous_character = Some(character);
					if word_index != last_part_index {
						word_index += 1;
					} else {
						match_end = i;
					}

					continue;
				}

				// If it does, and neither does the last character, reset the state:
				if !self.parts[word_index - 1].matches(character, previous_character) {
					word_index = 0;
					previous_character = None;
					break;
				}
			}

			if match_end != usize::MAX {
				matched = true;
				sentence.mark(match_start, match_end);
			}
		}

		matched
	}

	#[napi(js_name = "matches")]
	pub fn js_matches(&self, sentence: &mut Sentence) -> bool {
		self.matches(sentence)
	}

	#[napi(getter, js_name = "length")]
	pub fn js_length(&self) -> u32 {
		self.parts.len().try_into().unwrap()
	}

	#[napi]
	pub fn to_string(&self) -> Result<String, Error> {
		let mut word = String::new();

		if !self.bound_left {
			word.push_str("**");
		}

		for part in &self.parts {
			word.push_str(&part.to_string());
		}

		if !self.bound_right {
			word.push_str("**");
		}

		Ok(word)
	}
}
