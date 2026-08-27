import { addProjectWorkspace } from '@/state/projects'

/** Select a native directory and offer the restart needed to apply it. */
export async function switchWorkspace(path: string): Promise<boolean> {
  return addProjectWorkspace(path)
}
