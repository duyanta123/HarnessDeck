export const HARNESSDECK_PROTOCOL: 3
export const HARNESSDECK_HOST_PROTOCOL: 1

export interface HarnessDeckHostProfile {
  readonly name: string
  readonly dir: string
  readonly initialized: boolean
  readonly servesWindow: boolean
  readonly packages: number
  readonly problem: 'unreadable-manifest' | null
}

export interface HarnessDeckHost {
  readonly protocol: 1
  readonly studio: { readonly name: 'HarnessDeck'; readonly version: string }
  readonly harness: { readonly version: string }
  readonly platform: string
  readonly capabilities: readonly ('profiles.read' | 'runtime.read')[]
  readonly restrictions: {
    readonly arbitraryCommands: false
    readonly nativeHandles: false
    readonly packageMutation: false
    readonly profileMutation: false
  }
  readonly profiles: {
    readonly current: { readonly name: string; readonly dir: string }
    list(): readonly HarnessDeckHostProfile[]
  }
}

export interface HarnessDeckHostContext {
  readonly harnessDeckHost?: HarnessDeckHost | { readonly protocol?: unknown }
  get?(name: 'harnessDeckHost'): HarnessDeckHost | { readonly protocol?: unknown } | undefined
}

export interface HarnessDeckLink {
  readonly url: string
  readonly route: string
  readonly query: Readonly<Record<string, string>>
}

export type HarnessDeckCapability =
  'notify' | 'pick' | 'badge' | 'link' | 'profiles' | 'plugins' | 'workspace'

export interface HarnessDeckOffer {
  readonly protocol: 3
  readonly app: string
  readonly version: string
  readonly platform: string
  readonly scheme: 'harnessdeck'
  readonly capabilities: readonly HarnessDeckCapability[]
  readonly link: HarnessDeckLink | null
}

export interface HarnessDeckProfile {
  readonly name: string
  readonly dir: string
  readonly initialized: boolean
  readonly shipped: boolean
  readonly servesWindow: boolean
  readonly plugins: number
  readonly disabled: number
}

export interface HarnessDeckProfileRoster {
  readonly profiles: readonly HarnessDeckProfile[]
  readonly selected: string
  readonly root: string
}

export interface HarnessDeckInstalledPlugin {
  readonly name: string
  readonly spec: string
  readonly active: boolean
  readonly disabled: boolean
  readonly builtin: boolean
  readonly marketReceipt: string | null
}

export interface HarnessDeckPluginState {
  readonly profile: string
  readonly profileDir: string
  readonly initialized: boolean
  readonly plugins: readonly HarnessDeckInstalledPlugin[]
  readonly packageManager: boolean
}

export interface HarnessDeckWorkspaceAdmission {
  readonly allowed: boolean
  readonly reason: string
}

export interface HarnessDeckPickOptions {
  readonly mode?: 'open' | 'save' | 'directory'
  readonly title?: string
  readonly defaultPath?: string
  readonly filters?: readonly {
    readonly name: string
    readonly extensions: readonly string[]
  }[]
}

export interface HarnessDeck {
  readonly protocol: 3
  hello(): Promise<HarnessDeckOffer>
  notify(options: { readonly title: string; readonly body?: string }): Promise<void>
  pick(options?: HarnessDeckPickOptions): Promise<{ readonly path: string | null }>
  badge(count: number): Promise<void>
  readonly profiles: {
    list(): Promise<HarnessDeckProfileRoster>
    select(name: string): Promise<{
      readonly roster: HarnessDeckProfileRoster
      readonly restartRequired: true
    }>
  }
  readonly plugins: {
    install(request: {
      readonly name: string
      readonly version: string
      readonly displayName?: string
    }): Promise<HarnessDeckPluginState>
    remove(name: string): Promise<HarnessDeckPluginState>
  }
  readonly workspace: {
    validate(path: string): Promise<HarnessDeckWorkspaceAdmission>
    onDrop(handler: (path: string) => void): () => void
  }
  onLink(handler: (link: HarnessDeckLink) => void): () => void
}

export interface HarnessDeckScope {
  readonly harnessDeck?: HarnessDeck | { readonly protocol?: unknown }
}

export function getHarnessDeck(scope?: unknown): HarnessDeck | undefined
export function requireHarnessDeck(scope?: unknown): HarnessDeck
export function hasHarnessDeckCapability(
  offer: unknown,
  capability: string,
): capability is HarnessDeckCapability
export function onHarnessDeckWorkspaceDrop(
  handler: (path: string) => void,
  scope?: unknown,
): () => void
export function getHarnessDeckHost(ctx?: unknown): HarnessDeckHost | undefined
export function requireHarnessDeckHost(ctx?: unknown): HarnessDeckHost

declare global {
  interface Window {
    readonly harnessDeck?: HarnessDeck
  }
}
