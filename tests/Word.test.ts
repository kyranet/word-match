import { Sentence, Word } from '../index';

describe('Word', () => {
	test('GIVEN full bound word THEN matches full words only', () => {
		const word = new Word('bar');

		expect(word.boundLeft).toBe(true);
		expect(word.boundRight).toBe(true);
		expect(word.toString()).toBe('bar');
		expect(word.matches(new Sentence('bar'))).toBe(true);
		expect(word.matches(new Sentence('baz'))).toBe(false);
		expect(word.matches(new Sentence('barb'))).toBe(false);
		expect(word.matches(new Sentence('rebar'))).toBe(false);
		expect(word.matches(new Sentence('rebars'))).toBe(false);
	});

	test('GIVEN left unbound word THEN matches suffix words only', () => {
		const word = new Word('**bar');

		expect(word.boundLeft).toBe(false);
		expect(word.boundRight).toBe(true);
		expect(word.toString()).toBe('**bar');
		expect(word.matches(new Sentence('bar'))).toBe(true);
		expect(word.matches(new Sentence('baz'))).toBe(false);
		expect(word.matches(new Sentence('barb'))).toBe(false);
		expect(word.matches(new Sentence('rebar'))).toBe(true);
		expect(word.matches(new Sentence('rebars'))).toBe(false);
	});

	test('GIVEN right unbound word THEN matches prefix words only', () => {
		const word = new Word('bar**');

		expect(word.boundLeft).toBe(true);
		expect(word.boundRight).toBe(false);
		expect(word.toString()).toBe('bar**');
		expect(word.matches(new Sentence('bar'))).toBe(true);
		expect(word.matches(new Sentence('baz'))).toBe(false);
		expect(word.matches(new Sentence('barb'))).toBe(true);
		expect(word.matches(new Sentence('rebar'))).toBe(false);
		expect(word.matches(new Sentence('rebars'))).toBe(false);
	});

	test('GIVEN full unbound word THEN matches matching substrings only', () => {
		const word = new Word('**bar**');

		expect(word.boundLeft).toBe(false);
		expect(word.boundRight).toBe(false);
		expect(word.toString()).toBe('**bar**');
		expect(word.matches(new Sentence('bar'))).toBe(true);
		expect(word.matches(new Sentence('baz'))).toBe(false);
		expect(word.matches(new Sentence('barb'))).toBe(true);
		expect(word.matches(new Sentence('rebar'))).toBe(true);
		expect(word.matches(new Sentence('rebars'))).toBe(true);
	});

	test('GIVEN letter duplication THEN matches duplicated letters as well', () => {
		const word = new Word('bar');
		const original = 'I saw a [bbbaaaarrr]!';
		const sentence = new Sentence(original);

		expect(word.matches(sentence)).toBe(true);
		expect(sentence.toCensoredString({ original, character: '-' })).toBe('I saw a [----------]!');
	});

	test('GIVEN word split across different words THEN matches them correctly', () => {
		const word = new Word('bar');
		const original = 'Oh no! A b aaaa rrr!';
		const sentence = new Sentence(original);

		expect(word.matches(sentence)).toBe(true);
		expect(sentence.toCensoredString({ original, character: '-' })).toBe('Oh no! A - ---- ---!');
	});

	describe('edge cases', () => {
		test('GIVEN an empty group THEN gets omitted', () => {
			const word = new Word('foo[]');
			expect(word.toString()).toBe('foo');
		});

		test('GIVEN an empty group with a single character THEN becomes a non-group single character', () => {
			const word = new Word('fo[o]');
			expect(word.toString()).toBe('foo');
		});

		test('GIVEN an unclosed group THEN throws', () => {
			expect(() => new Word('[bar')).toThrowError(new Error('Unterminated character group'));
		});

		test('GIVEN an escape character at the end THEN throws', () => {
			expect(() => new Word('bar\\')).toThrowError(new Error('Escape character cannot be at the end of the word'));
		});

		test('GIVEN an empty word THEN throws', () => {
			expect(() => new Word('')).toThrowError(new Error('The word cannot be empty'));
		});

		test('GIVEN an empty word with unbound start THEN throws', () => {
			expect(() => new Word('**')).toThrowError(new Error('Wildcards cannot be the only character in the word'));
		});

		test('GIVEN an empty word with unbound start and end THEN throws', () => {
			expect(() => new Word('****')).toThrowError(new Error('Wildcards cannot be the only character in the word'));
		});
	});
});
