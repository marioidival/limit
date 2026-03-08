# Plan: Fix Text Wrapping for Long Input and Chat Messages

## Problem Statement

Long text in input field and chat messages disappears when it exceeds the available width. Text should wrap to multiple lines instead of being truncated.

## Root Cause Analysis

### 1. InputPrompt Component (`limit-tui/src/components/prompt.rs`)

**Location:** `render()` method around lines 174-226

**Current Code:**
```rust
let paragraph = Paragraph::new(display_text);
paragraph.render(area, buf);
```

**Issue:** No `wrap(Wrap { trim: false })` is applied. Text is rendered on a single line and truncated if too long.

**Impact:** User cannot see what they're typing when the input exceeds terminal width.

---

### 2. TuiApp Input Area (`limit-cli/src/tui_bridge.rs`)

**Location:** `draw_ui()` method around lines 618-640

**Current Code:**
```rust
let input_para = Paragraph::new(input_line);
f.render_widget(input_para, input_inner);
```

**Issue:** Similar to InputPrompt, no wrapping is applied.

**Impact:** Same as above - long input text disappears.

---

### 3. ChatView (`limit-tui/src/components/chat.rs`)

**Location:** Multiple render locations

**Status:** Already has wrapping implemented with `Wrap { trim: false }`.

**Note:** Chat messages should work correctly, but we should verify the behavior with very long single words or code blocks.

## Proposed Solution

### Phase 1: Fix InputPrompt Component

**File:** `limit-tui/src/components/prompt.rs`

**Change 1: Add Wrapping**
```rust
// In render() method, around line 223
let paragraph = Paragraph::new(display_text)
    .wrap(Wrap { trim: false });
paragraph.render(area, buf);
```

**Change 2: Handle Cursor Position with Wrapping**

When text wraps to multiple lines, the cursor position (single integer) becomes ambiguous. We need to:

1. **Track cursor by line and column:**
   ```rust
   struct CursorPosition {
       line: usize,
       column: usize,
   }
   ```

2. **Calculate cursor screen position:**
   - Determine which line the cursor is on after wrapping
   - Calculate the column position within that line
   - Use ratatui's cursor rendering or manual highlighting

3. **Update navigation:**
   - Arrow Up/Down: Move between wrapped lines
   - Arrow Left/Right: Move within current line, crossing line boundaries
   - Home/End: Move to start/end of current wrapped line

**Implementation Strategy:**

Option A: Use ratatui's built-in cursor (simpler, less control)
- Hide terminal cursor
- Render cursor as reversed text at calculated position
- Only works for display, not for actual cursor movement

Option B: Calculate visual cursor position (more control, more complex)
- Implement `visual_cursor_position(area_width: u16) -> (u16, u16)`
- Returns (x, y) coordinates within the render area
- Use `f.set_cursor(x + area.x, y + area.y)` in parent
- Requires parent component to handle cursor rendering

**Recommendation:** Start with Option A (reversed text cursor) as it's self-contained and doesn't require changes to parent components.

---

### Phase 2: Fix TuiApp Input Area

**File:** `limit-cli/src/tui_bridge.rs`

**Change 1: Add Wrapping**
```rust
// In draw_ui() method, around line 640
let input_para = Paragraph::new(input_line)
    .wrap(Wrap { trim: false });
f.render_widget(input_para, input_inner);
```

**Change 2: Multi-line Cursor Handling**

Similar to InputPrompt, we need to handle cursor position when text wraps.

**Current cursor rendering (lines 618-639):**
```rust
let before_cursor = &input_text[..cursor_pos];
let at_cursor = &input_text[cursor_pos..cursor_pos + char_len];
let after_cursor = &input_text[cursor_pos + char_len..];

let cursor_style = if cursor_blink_state {
    Style::default().bg(Color::White).fg(Color::Black)
} else {
    Style::default().bg(Color::Reset).fg(Color::Reset)
};

let input_line = Line::from(vec![
    Span::raw(before_cursor),
    Span::styled(at_cursor, cursor_style),
    Span::raw(after_cursor),
]);
```

**Issue:** This assumes cursor is at a single position. When text wraps, this approach doesn't work.

**Solution:** Implement a multi-line cursor renderer:

```rust
// Calculate wrapped lines
let wrapped_lines = wrap_text(input_text, area_width);

// Find which line contains the cursor
let (cursor_line_idx, cursor_col_idx) = find_cursor_line(&wrapped_lines, cursor_pos);

// Build lines with cursor highlighting
let lines: Vec<Line> = wrapped_lines.iter().enumerate().map(|(i, line_text)| {
    if i == cursor_line_idx {
        // This line has the cursor - highlight the character
        let (before, cursor_char, after) = split_at_char(line_text, cursor_col_idx);
        Line::from(vec![
            Span::raw(before),
            Span::styled(cursor_char.to_string(), cursor_style),
            Span::raw(after),
        ])
    } else {
        // Normal line without cursor
        Line::from(vec![Span::raw(line_text)])
    }
}).collect();

let input_para = Paragraph::new(Text::from(lines))
    .wrap(Wrap { trim: false });
```

---

### Phase 3: Verify ChatView

**File:** `limit-tui/src/components/chat.rs`

**Action:** Verify that wrapping works correctly for all message types:
- Plain text
- Code blocks
- Long single words
- Unicode characters
- Mixed content

**Potential Issues to Check:**
1. `estimate_line_count()` function (around line 289) may not account for all cases
2. Code blocks with very long lines may not wrap properly
3. Syntax highlighting might interfere with wrapping

---

## Implementation Steps

1. **Step 1: Test and document current behavior**
   - Create test cases with long input text
   - Document how text currently behaves (screenshot or description)

2. **Step 2: Implement InputPrompt wrapping**
   - Add `Wrap { trim: false }` to Paragraph
   - Test that text wraps correctly
   - Cursor may appear on wrong line initially (expected)

3. **Step 3: Implement cursor positioning for wrapped text**
   - Add function to calculate cursor line and column
   - Update render to highlight cursor on correct line
   - Test cursor movement with arrow keys

4. **Step 4: Implement TuiApp input area wrapping**
   - Apply same changes as InputPrompt
   - Test in actual TUI application

5. **Step 5: Handle edge cases**
   - Empty input with wrap enabled
   - Single very long word (should split)
   - Unicode characters (multi-byte)
   - Cursor at beginning/end of input
   - Cursor in middle of very long input

6. **Step 6: Verify ChatView behavior**
   - Test with very long messages
   - Test with code blocks containing long lines
   - Fix any issues found

7. **Step 7: Add tests**
   - Unit tests for cursor position calculation
   - Integration tests for rendering wrapped text
   - Manual testing checklist

---

## Considerations

### Performance
- Wrapping calculation on every render could be expensive for very long text
- Consider caching wrapped lines or incremental updates

### User Experience
- Should we limit input length (e.g., 10,000 characters)?
- Should we show an indicator when input is truncated?
- How to handle scrolling in multi-line input?

### Backward Compatibility
- Changes should not break existing functionality
- Single-line mode could be kept as an option if needed

### Accessibility
- Ensure cursor is visible and clearly indicates insertion point
- Consider cursor blink with multi-line text

---

## Alternative Approaches Considered

### 1. Limit Input Length
**Pros:** Simple, no wrapping needed
**Cons:** Arbitrary limit, user frustration
**Verdict:** Not recommended

### 2. Scroll Input Field
**Pros:** No wrapping complexity
**Cons:** User can't see what they typed, confusing
**Verdict:** Last resort only

### 3. Use Textarea Widget
**Pros:** Built for multi-line input
**Cons:** Requires external crate or custom implementation
**Verdict:** Consider for future, but fix current implementation first

---

## Success Criteria

- [ ] Input text wraps to multiple lines when exceeding terminal width
- [ ] Cursor is visible at the correct position in wrapped text
- [ ] Arrow keys navigate correctly through wrapped lines
- [ ] Chat messages with long content display completely
- [ ] Code blocks with long lines wrap appropriately
- [ ] No text is lost or truncated
- [ ] Performance remains acceptable (no noticeable lag)
- [ ] Tests cover new functionality
