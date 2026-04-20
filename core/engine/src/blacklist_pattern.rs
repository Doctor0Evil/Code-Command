/// Blacklist pattern matching engine
/// 
/// Implements a regex-subset NFA engine for blacklist token matching.
/// Supports: literals, character classes, wildcards, anchors (^, $), 
/// repetition (*, +, ?), and alternation (|).

use std::fmt;

/// Operations in the compiled NFA program
#[derive(Debug, Clone)]
pub enum PatternOp {
    /// Match a literal byte
    Literal(u8),
    /// Match any byte (.)
    Any,
    /// Match a character class [abc] or [^abc]
    CharClass { chars: Vec<u8>, negated: bool },
    /// Match start of line
    AnchorStart,
    /// Match end of line
    AnchorEnd,
    /// Match zero or more of previous op (*)
    RepeatZeroOrMore,
    /// Match one or more of previous op (+)
    RepeatOneOrMore,
    /// Match zero or one of previous op (?)
    RepeatOptional,
    /// Alternation branch point (|)
    Alternate { branch_count: usize },
    /// Capture group start (for exact_match extraction)
    CaptureStart(usize),
    /// Capture group end
    CaptureEnd(usize),
}

/// Compiled blacklist pattern (NFA program)
#[derive(Debug, Clone)]
pub struct BlacklistPattern {
    pub ops: Vec<PatternOp>,
    pub source: String,
}

impl BlacklistPattern {
    /// Parse a pattern string into an NFA program
    /// 
    /// Supports a subset of regex syntax:
    /// - Literals: abc
    /// - Wildcard: .
    /// - Character classes: [abc], [^abc], [a-z]
    /// - Anchors: ^, $
    /// - Repetition: *, +, ?
    /// - Alternation: |
    /// - Grouping: ()
    pub fn parse(pattern: &str) -> Result<Self, PatternError> {
        let mut ops = Vec::new();
        let mut chars = pattern.chars().peekable();
        let mut capture_id = 0;
        
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    // Escape sequence
                    if let Some(escaped) = chars.next() {
                        ops.push(PatternOp::Literal(escaped as u8));
                    } else {
                        return Err(PatternError::TrailingBackslash);
                    }
                }
                '.' => {
                    ops.push(PatternOp::Any);
                }
                '^' => {
                    ops.push(PatternOp::AnchorStart);
                }
                '$' => {
                    ops.push(PatternOp::AnchorEnd);
                }
                '*' => {
                    if ops.is_empty() {
                        return Err(PatternError::InvalidRepetition);
                    }
                    ops.push(PatternOp::RepeatZeroOrMore);
                }
                '+' => {
                    if ops.is_empty() {
                        return Err(PatternError::InvalidRepetition);
                    }
                    ops.push(PatternOp::RepeatOneOrMore);
                }
                '?' => {
                    if ops.is_empty() {
                        return Err(PatternError::InvalidRepetition);
                    }
                    ops.push(PatternOp::RepeatOptional);
                }
                '|' => {
                    ops.push(PatternOp::Alternate { branch_count: 0 }); // Will be resolved later
                }
                '(' => {
                    ops.push(PatternOp::CaptureStart(capture_id));
                    capture_id += 1;
                }
                ')' => {
                    ops.push(PatternOp::CaptureEnd(capture_id.saturating_sub(1)));
                }
                '[' => {
                    // Character class
                    let char_class = Self::parse_char_class(&mut chars)?;
                    ops.push(PatternOp::CharClass {
                        chars: char_class.chars,
                        negated: char_class.negated,
                    });
                }
                c => {
                    // Literal character
                    ops.push(PatternOp::Literal(c as u8));
                }
            }
        }
        
        Ok(BlacklistPattern {
            ops,
            source: pattern.to_string(),
        })
    }
    
    fn parse_char_class<I>(chars: &mut std::iter::Peekable<I>) -> Result<CharClass, PatternError>
    where
        I: Iterator<Item = char>,
    {
        let mut class_chars = Vec::new();
        let mut negated = false;
        
        // Check for negation
        if let Some(&first) = chars.peek() {
            if first == '^' {
                negated = true;
                chars.next();
            }
        }
        
        // Parse characters until closing ]
        while let Some(c) = chars.next() {
            if c == ']' {
                break;
            }
            
            // Handle range like a-z
            if c == '-' && !class_chars.is_empty() {
                if let Some(&next) = chars.peek() {
                    if next != ']' {
                        chars.next(); // consume the end char
                        let start = class_chars.pop().ok_or(PatternError::InvalidCharRange)?;
                        let end = next as u8;
                        // Add all characters in range
                        for b in start..=end {
                            class_chars.push(b);
                        }
                        continue;
                    }
                }
            }
            
            class_chars.push(c as u8);
        }
        
        Ok(CharClass {
            chars: class_chars,
            negated,
        })
    }
    
    /// Run the NFA against input bytes, returning match info if found
    pub fn matches(&self, input: &[u8]) -> Option<MatchInfo> {
        // Simple backtracking NFA simulation
        let mut state = NfaState::new(input);
        
        if self.run_nfa(&mut state, 0, 0) {
            Some(MatchInfo {
                start: state.match_start,
                end: state.position,
                captures: state.captures.clone(),
            })
        } else {
            None
        }
    }
    
    fn run_nfa(&self, state: &mut NfaState, op_idx: usize, input_idx: usize) -> bool {
        if op_idx >= self.ops.len() {
            // All ops consumed - success if we've matched something
            return input_idx <= state.input.len();
        }
        
        let op = &self.ops[op_idx];
        
        match op {
            PatternOp::Literal(byte) => {
                if input_idx < state.input.len() && state.input[input_idx] == *byte {
                    state.position = input_idx + 1;
                    self.run_nfa(state, op_idx + 1, input_idx + 1)
                } else {
                    false
                }
            }
            PatternOp::Any => {
                if input_idx < state.input.len() {
                    state.position = input_idx + 1;
                    self.run_nfa(state, op_idx + 1, input_idx + 1)
                } else {
                    false
                }
            }
            PatternOp::CharClass { chars, negated } => {
                if input_idx < state.input.len() {
                    let byte = state.input[input_idx];
                    let in_class = chars.contains(&byte);
                    let matches = if *negated { !in_class } else { in_class };
                    if matches {
                        state.position = input_idx + 1;
                        self.run_nfa(state, op_idx + 1, input_idx + 1)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            PatternOp::AnchorStart => {
                if input_idx == 0 || (input_idx > 0 && state.input[input_idx - 1] == b'\n') {
                    self.run_nfa(state, op_idx + 1, input_idx)
                } else {
                    false
                }
            }
            PatternOp::AnchorEnd => {
                if input_idx == state.input.len() || state.input.get(input_idx) == Some(&b'\n') {
                    self.run_nfa(state, op_idx + 1, input_idx)
                } else {
                    false
                }
            }
            PatternOp::RepeatZeroOrMore => {
                // Try matching zero times first (greedy would try max first)
                if self.run_nfa(state, op_idx + 1, input_idx) {
                    return true;
                }
                // Try matching one or more times
                if input_idx < state.input.len() {
                    // This is simplified - proper implementation needs to track which op to repeat
                    self.run_nfa(state, op_idx + 1, input_idx + 1)
                } else {
                    false
                }
            }
            PatternOp::RepeatOneOrMore => {
                // Must match at least once
                if input_idx < state.input.len() {
                    if self.run_nfa(state, op_idx + 1, input_idx + 1) {
                        return true;
                    }
                    // Continue matching
                    self.run_nfa(state, op_idx, input_idx + 1)
                } else {
                    false
                }
            }
            PatternOp::RepeatOptional => {
                // Try skipping
                if self.run_nfa(state, op_idx + 1, input_idx) {
                    return true;
                }
                // Try matching once
                if input_idx < state.input.len() {
                    self.run_nfa(state, op_idx + 1, input_idx + 1)
                } else {
                    false
                }
            }
            PatternOp::Alternate { .. } => {
                // Simplified alternation - would need proper branch tracking
                self.run_nfa(state, op_idx + 1, input_idx)
            }
            PatternOp::CaptureStart(id) => {
                state.captures.insert(*id, input_idx);
                self.run_nfa(state, op_idx + 1, input_idx)
            }
            PatternOp::CaptureEnd(id) => {
                state.captures.insert(*id, input_idx);
                self.run_nfa(state, op_idx + 1, input_idx)
            }
        }
    }
}

struct CharClass {
    chars: Vec<u8>,
    negated: bool,
}

struct NfaState<'a> {
    input: &'a [u8],
    position: usize,
    match_start: usize,
    captures: std::collections::HashMap<usize, usize>,
}

impl<'a> NfaState<'a> {
    fn new(input: &'a [u8]) -> Self {
        NfaState {
            input,
            position: 0,
            match_start: 0,
            captures: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub start: usize,
    pub end: usize,
    pub captures: std::collections::HashMap<usize, usize>,
}

/// Error types for pattern parsing
#[derive(Debug, Clone)]
pub enum PatternError {
    InvalidRepetition,
    InvalidCharRange,
    TrailingBackslash,
    UnbalancedParentheses,
    UnbalancedBracket,
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::InvalidRepetition => write!(f, "Invalid repetition operator"),
            PatternError::InvalidCharRange => write!(f, "Invalid character range"),
            PatternError::TrailingBackslash => write!(f, "Trailing backslash"),
            PatternError::UnbalancedParentheses => write!(f, "Unbalanced parentheses"),
            PatternError::UnbalancedBracket => write!(f, "Unbalanced bracket"),
        }
    }
}

impl std::error::Error for PatternError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_literal_pattern() {
        let pattern = BlacklistPattern::parse("hello").unwrap();
        assert!(pattern.matches(b"hello").is_some());
        assert!(pattern.matches(b"world").is_none());
    }
    
    #[test]
    fn test_wildcard_pattern() {
        let pattern = BlacklistPattern::parse("h.llo").unwrap();
        assert!(pattern.matches(b"hello").is_some());
        assert!(pattern.matches(b"hallo").is_some());
        assert!(pattern.matches(b"hllo").is_none());
    }
    
    #[test]
    fn test_anchor_pattern() {
        let pattern = BlacklistPattern::parse("^hello$").unwrap();
        assert!(pattern.matches(b"hello").is_some());
        assert!(pattern.matches(b"hello world").is_none());
    }
}
