import { nextTestSetup } from 'e2e-utils'
import { retry } from 'next-test-utils'

describe('rsc-client-disconnect-cache-signal', () => {
  const { next } = nextTestSetup({
    files: __dirname,
  })

  async function disconnectAndExpectAbort(
    id: string,
    { streaming, rsc }: { streaming: boolean; rsc: boolean }
  ) {
    const controller = new AbortController()
    const res = next.fetch(`/?id=${id}${streaming ? '&streaming=1' : ''}`, {
      headers: rsc ? { RSC: '1' } : {},
      signal: controller.signal,
    })

    if (streaming) {
      const decoder = new TextDecoder()
      for await (const rawChunk of (await res).body) {
        const chunk =
          typeof rawChunk === 'string' ? rawChunk : decoder.decode(rawChunk)
        if (chunk.includes('stream-started')) {
          break
        }
      }
    } else {
      res.catch(() => {})
      await retry(async () => {
        expect(next.cliOutput).toContain(`hang-started ${id}`)
      })
    }

    controller.abort()

    await retry(async () => {
      expect(next.cliOutput).toContain(`cache-signal-aborted ${id}`)
    })

    // Give any error from the aborted render a chance to be logged.
    expect(await (await next.fetch('/ok')).text()).toContain('ok')
    expect(next.cliOutput).not.toContain(
      'An error occurred in the Server Components render'
    )
  }

  it('aborts cacheSignal when the client disconnects from an RSC request', async () => {
    await disconnectAndExpectAbort('rsc-request', {
      streaming: true,
      rsc: true,
    })
  })

  it('aborts cacheSignal when the client disconnects before the HTML shell is sent', async () => {
    await disconnectAndExpectAbort('html-before-shell', {
      streaming: false,
      rsc: false,
    })
  })

  it('aborts cacheSignal when the client disconnects after the HTML shell is sent', async () => {
    await disconnectAndExpectAbort('html-after-shell', {
      streaming: true,
      rsc: false,
    })
  })
})
