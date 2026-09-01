import { getHarnessDeck, hasHarnessDeckCapability } from '@bugcome/harnessdeck-sdk'

/** Mount an optional Desktop adapter without changing ordinary Harness use. */
export async function mountDesktopExample(scope = window) {
  const desktop = getHarnessDeck(scope)
  if (!desktop) return () => {}

  const offer = await desktop.hello()
  if (!hasHarnessDeckCapability(offer, 'workspace')) return () => {}

  return desktop.workspace.onDrop((path) => {
    // Production plugins normally pass this path to their upstream Harness
    // workspace service. The SDK deliberately does not expose raw filesystem IO.
    scope.dispatchEvent(new CustomEvent('example:workspace-drop', { detail: path }))
  })
}
