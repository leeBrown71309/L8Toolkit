interface TextRun {
  text: string;
  matched: boolean;
}

/** Groups characters into runs so each match renders as one <mark>. */
function toRuns(text: string, indices: number[]): TextRun[] {
  const matched = new Set(indices);
  const runs: TextRun[] = [];
  // Array.from splits by code point, matching the Rust char indices.
  Array.from(text).forEach((character, index) => {
    const isMatch = matched.has(index);
    const last = runs[runs.length - 1];
    if (last && last.matched === isMatch) last.text += character;
    else runs.push({ text: character, matched: isMatch });
  });
  return runs;
}

export function HighlightedText({ text, indices }: { text: string; indices: number[] }) {
  if (indices.length === 0) return <>{text}</>;
  return (
    <>
      {toRuns(text, indices).map((run, index) =>
        run.matched ? <mark key={index}>{run.text}</mark> : <span key={index}>{run.text}</span>,
      )}
    </>
  );
}
