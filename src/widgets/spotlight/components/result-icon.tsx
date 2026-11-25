import type { SearchResult } from "../../../shared/types";
import { useShellIcon } from "../../../shared/use-shell-icon";
import { glyphFor } from "../result-presentation";

const GLYPH_TO_TILE_RATIO = 0.56;

export function ResultIcon({ result, size }: { result: SearchResult; size: number }) {
  const icon = useShellIcon(result.iconTarget);

  if (icon) {
    return (
      <img className="result-icon result-icon--image" src={icon} width={size} height={size} alt="" draggable={false} />
    );
  }

  const Glyph = glyphFor(result);
  return (
    <span className="result-icon result-icon--glyph" style={{ width: size, height: size }} aria-hidden="true">
      <Glyph size={Math.round(size * GLYPH_TO_TILE_RATIO)} />
    </span>
  );
}
