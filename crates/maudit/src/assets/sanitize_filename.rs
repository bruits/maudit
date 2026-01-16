// MIT License

// Copyright (c) 2024-present VoidZero Inc. & Contributors

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

macro_rules! matches_invalid_chars {
  ($chars:ident) => {
    matches!($chars,
      '\u{0000}'
        ..='\u{001f}'
          | '"'
          | '#'
          | '$'
          | '%'
          | '&'
          | '*'
          | '+'
          | ','
          | ':'
          | ';'
          | '<'
          | '='
          | '>'
          | '?'
          | '['
          | ']'
          | '^'
          | '`'
          | '{'
          | '|'
          | '}'
          | '\u{007f}'
    )
  };
}

// Follow from https://github.com/rollup/rollup/blob/master/src/utils/sanitizeFileName.ts
pub fn default_sanitize_file_name(str: &str) -> String {
    let mut sanitized = String::with_capacity(str.len());
    let mut chars = str.chars();

    // A `:` is only allowed as part of a windows drive letter (ex: C:\foo)
    // Otherwise, avoid them because they can refer to NTFS alternate data streams.
    if starts_with_windows_drive(str) {
        sanitized.push(chars.next().unwrap());
        sanitized.push(chars.next().unwrap());
    }

    for char in chars {
        if matches_invalid_chars!(char) {
            sanitized.push('_');
        } else {
            sanitized.push(char);
        }
    }
    sanitized
}

fn starts_with_windows_drive(str: &str) -> bool {
    let mut chars = str.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    chars.next().is_some_and(|c| c == ':')
}

#[test]
fn test_sanitize_file_name() {
    assert_eq!(default_sanitize_file_name("\0+a=Z_0-"), "__a_Z_0-");
}
