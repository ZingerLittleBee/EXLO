export function ScanlineOverlay() {
  return (
    <div
      className="pointer-events-none fixed inset-0 z-50"
      style={{
        background: `repeating-linear-gradient(
          0deg,
          transparent,
          transparent 2px,
          rgba(0, 0, 0, 0.05) 2px,
          rgba(0, 0, 0, 0.05) 4px
        )`
      }}
    />
  )
}
