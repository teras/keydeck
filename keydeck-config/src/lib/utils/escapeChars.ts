/**
 * Processes escape sequences in a string for display purposes.
 * Matches the backend implementation in src/keyboard.rs
 *
 * Supported escape sequences:
 * - \n -> newline
 * - \t -> tab
 * - \r -> carriage return (treated as newline)
 * - \\ -> backslash
 * - \e -> escape character
 */
export function processEscapeSequences(text: string): string {
  if (!text) return '';

  let result = '';
  let i = 0;

  while (i < text.length) {
    if (text[i] === '\\' && i + 1 < text.length) {
      const nextChar = text[i + 1];
      switch (nextChar) {
        case 'n':
          result += '\n';
          i += 2;
          break;
        case 't':
          result += '\t';
          i += 2;
          break;
        case 'r':
          result += '\n'; // Treat \r as newline for display
          i += 2;
          break;
        case '\\':
          result += '\\';
          i += 2;
          break;
        case 'e':
          result += '\x1b'; // Escape character (probably won't display visibly)
          i += 2;
          break;
        default:
          // Unknown escape sequence, keep the backslash
          result += text[i];
          i += 1;
          break;
      }
    } else {
      result += text[i];
      i += 1;
    }
  }

  return result;
}
