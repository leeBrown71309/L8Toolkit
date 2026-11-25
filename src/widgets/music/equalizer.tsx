const BAR_COUNT = 4;

/**
 * Four bouncing bars: the universal "something is playing" sign. They
 * freeze (rather than vanish) on pause, so the state change reads clearly.
 */
export function Equalizer({ playing, className = "" }: { playing: boolean; className?: string }) {
  return (
    <span className={`equalizer ${className}`} data-playing={playing} aria-hidden="true">
      {Array.from({ length: BAR_COUNT }, (_, index) => (
        <span key={index} className="equalizer__bar" />
      ))}
    </span>
  );
}
