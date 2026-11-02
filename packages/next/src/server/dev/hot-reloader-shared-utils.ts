import { getPathMatch } from '../../shared/lib/router/utils/path-match'
import { parseVersionInfo, type VersionInfo } from './parse-version-info'

export const matchNextPageBundleRequest = getPathMatch(
  '/_next/static/chunks/pages/:path*.js(\\.map|)'
)
/**
 * Version info to send while the npm registry request from `getVersionInfo()`
 * is still pending. Never await that request before talking to the browser.
 * A slow registry would otherwise delay the dev overlay.
 */
export function getPendingVersionInfo(): VersionInfo {
  return {
    installed: process.env.__NEXT_VERSION as string,
    staleness: 'unknown',
  }
}

export async function getVersionInfo(): Promise<VersionInfo> {
  const installed = process.env.__NEXT_VERSION as string

  try {
    let res

    try {
      // use NPM registry regardless user using Yarn
      res = await fetch('https://registry.npmjs.org/-/package/next/dist-tags')
    } catch {
      // ignore fetch errors
    }

    if (!res || !res.ok) return { installed, staleness: 'unknown' }

    const { latest, canary } = await res.json()

    return parseVersionInfo({ installed, latest, canary })
  } catch (e: any) {
    console.error(e)
    return { installed, staleness: 'unknown' }
  }
}
