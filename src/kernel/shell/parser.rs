// AIOS Shell Parser
//
// Model: opencode/minimax-m2.5-free
// Tool: opencode
// Prompt: Command parsing utilities for AIOS shell.

use crate::shell::MAX_ARGS;

pub const MAX_TOKENS: usize = 32;
pub const TOKEN_DELIMITERS: &[u8] = b" \t\n\r";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Word,
    RedirectIn,
    RedirectOut,
    RedirectAppend,
    Pipe,
    Background,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub const fn new() -> Self {
        Self {
            token_type: TokenType::None,
            start: 0,
            end: 0,
        }
    }
}

impl Default for Token {
    fn default() -> Self {
        Self::new()
    }
}

pub fn tokenize(input: &str) -> ([Token; MAX_TOKENS], usize) {
    let mut tokens: [Token; MAX_TOKENS] = [Token::new(); MAX_TOKENS];
    let mut count = 0;
    let mut pos = 0;
    let input_bytes = input.as_bytes();

    while pos < input_bytes.len() && count < MAX_TOKENS {
        while pos < input_bytes.len() && TOKEN_DELIMITERS.contains(&input_bytes[pos]) {
            pos += 1;
        }

        if pos >= input_bytes.len() {
            break;
        }

        let start = pos;
        let (token_type, new_pos) = match input_bytes[pos] {
            b'<' => (TokenType::RedirectIn, pos + 1),
            b'>' => {
                if pos + 1 < input_bytes.len() && input_bytes[pos + 1] == b'>' {
                    (TokenType::RedirectAppend, pos + 2)
                } else {
                    (TokenType::RedirectOut, pos + 1)
                }
            }
            b'|' => (TokenType::Pipe, pos + 1),
            b'&' => (TokenType::Background, pos + 1),
            _ => {
                let mut end = pos;
                while end < input_bytes.len()
                    && !TOKEN_DELIMITERS.contains(&input_bytes[end])
                    && input_bytes[end] != b'<'
                    && input_bytes[end] != b'>'
                    && input_bytes[end] != b'|'
                    && input_bytes[end] != b'&'
                {
                    end += 1;
                }
                (TokenType::Word, end)
            }
        };

        tokens[count] = Token {
            token_type,
            start,
            end: new_pos,
        };
        count += 1;
        pos = new_pos;
    }

    (tokens, count)
}

pub fn split_command_args(input: &str) -> ([&str; MAX_ARGS], usize) {
    let mut args: [&str; MAX_ARGS] = [
        "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
    ];
    let mut count = 0;

    for token in input.split_whitespace() {
        if count >= MAX_ARGS {
            break;
        }
        args[count] = token;
        count += 1;
    }

    (args, count)
}

pub fn parse_redirection<'a>(
    tokens: &'a [Token],
    input: &'a str,
) -> (Option<&'a str>, Option<&'a str>, bool) {
    let mut in_file: Option<&str> = None;
    let mut out_file: Option<&str> = None;
    let mut append = false;
    let input_bytes = input.as_bytes();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].token_type {
            TokenType::RedirectIn => {
                if i + 1 < tokens.len() && tokens[i + 1].token_type == TokenType::Word {
                    let start = tokens[i + 1].start;
                    let end = tokens[i + 1].end;
                    if let Ok(s) = core::str::from_utf8(&input_bytes[start..end]) {
                        in_file = Some(s);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            TokenType::RedirectOut => {
                if i + 1 < tokens.len() && tokens[i + 1].token_type == TokenType::Word {
                    let start = tokens[i + 1].start;
                    let end = tokens[i + 1].end;
                    if let Ok(s) = core::str::from_utf8(&input_bytes[start..end]) {
                        out_file = Some(s);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            TokenType::RedirectAppend => {
                append = true;
                if i + 1 < tokens.len() && tokens[i + 1].token_type == TokenType::Word {
                    let start = tokens[i + 1].start;
                    let end = tokens[i + 1].end;
                    if let Ok(s) = core::str::from_utf8(&input_bytes[start..end]) {
                        out_file = Some(s);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    (in_file, out_file, append)
}

pub fn is_background(tokens: &[Token]) -> bool {
    tokens.iter().any(|t| t.token_type == TokenType::Background)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let (tokens, count) = tokenize("ls -la");
        assert!(count >= 2);
    }

    #[test]
    fn test_tokenize_empty() {
        let (_, count) = tokenize("");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_tokenize_with_redirect() {
        let (_, count) = tokenize("cat < input.txt > output.txt");
        assert!(count >= 5);
    }

    #[test]
    fn test_tokenize_with_pipe() {
        let (_, count) = tokenize("ls | grep test");
        assert!(count >= 3);
    }

    #[test]
    fn test_split_command_args() {
        let (args, count) = split_command_args("echo hello world");
        assert_eq!(count, 3);
        assert_eq!(args[0], "echo");
        assert_eq!(args[1], "hello");
        assert_eq!(args[2], "world");
    }

    #[test]
    fn test_split_command_args_empty() {
        let (args, count) = split_command_args("");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_token_type_word() {
        let token = Token::new();
        assert_eq!(token.token_type, TokenType::None);
    }

    #[test]
    fn test_parse_redirection_input() {
        let (tokens, count) = tokenize("cat < input.txt");
        let (in_file, _, _) = parse_redirection(&tokens[..count], "cat < input.txt");
        assert!(in_file.is_some());
    }

    #[test]
    fn test_parse_redirection_output() {
        let (tokens, count) = tokenize("echo hello > output.txt");
        let (_, out_file, _) = parse_redirection(&tokens[..count], "echo hello > output.txt");
        assert!(out_file.is_some());
    }

    #[test]
    fn test_parse_redirection_append() {
        let (tokens, count) = tokenize("echo hello >> output.txt");
        let (_, out_file, append) = parse_redirection(&tokens[..count], "echo hello >> output.txt");
        assert!(out_file.is_some());
        assert!(append);
    }

    #[test]
    fn test_is_background() {
        let (tokens, count) = tokenize("ls &");
        assert!(is_background(&tokens[..count]));
    }

    #[test]
    fn test_is_background_no_ampersand() {
        let (tokens, count) = tokenize("ls");
        assert!(!is_background(&tokens[..count]));
    }

    #[test]
    fn test_tokenize_whitespace_only() {
        let (_, count) = tokenize("   \t\n   ");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_token_new() {
        let token = Token::new();
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 0);
        assert_eq!(token.token_type, TokenType::None);
    }

    #[test]
    fn test_token_delimiters() {
        assert!(TOKEN_DELIMITERS.contains(&b' '));
        assert!(TOKEN_DELIMITERS.contains(&b'\t'));
        assert!(TOKEN_DELIMITERS.contains(&b'\n'));
    }
}
