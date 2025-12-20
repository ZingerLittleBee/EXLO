export function ScanlineOverlay() {
  return (
    <div
      className="fixed inset-0 pointer-events-none z-50"
      style={{
        background: `repeating-linear-gradient(
          0deg,
          transparent,
          transparent 2px,
          rgba(0, 0, 0, 0.05) 2px,
          rgba(0, 0, 0, 0.05) 4px
        )`,
      }}
    />
  )
}
