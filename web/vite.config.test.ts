// @vitest-environment node
import { createServer as createHttpServer, type Server } from 'node:http'
import { fileURLToPath } from 'node:url'
import { build, createServer, type ProxyOptions } from 'vite'
import { afterEach, describe, expect, it } from 'vitest'
import config from './vite.config'

const configFile = fileURLToPath(new URL('./vite.config.ts', import.meta.url))

function apiProxy(): ProxyOptions {
  const entry = config.server?.proxy?.['/api']
  expect(entry, 'server.proxy["/api"]').toBeTypeOf('object')
  return entry as ProxyOptions
}

const closers: Array<() => Promise<void>> = []

afterEach(async () => {
  for (let close = closers.pop(); close !== undefined; close = closers.pop()) await close()
})

function portOf(server: Server | null): number {
  const address = server?.address()
  if (address === null || address === undefined || typeof address === 'string') {
    throw new Error('server is not listening on a TCP port')
  }
  return address.port
}

function listen(server: Server): Promise<number> {
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      resolve(portOf(server))
    })
  })
}

function close(server: Server): Promise<void> {
  return new Promise((resolve) => {
    server.close(() => {
      resolve()
    })
  })
}

describe('dev proxy (acceptance 5)', () => {
  it('proxies /api to http://127.0.0.1:7480 with changeOrigin', () => {
    const proxy = apiProxy()
    expect(proxy.target).toBe('http://127.0.0.1:7480')
    expect(proxy.changeOrigin).toBe(true)
  })

  it('the proxied request carries the upstream Host, not the dev server one', async () => {
    const seen: Array<string | undefined> = []
    const upstream = createHttpServer((req, res) => {
      seen.push(req.headers.host)
      res.writeHead(200, { 'content-type': 'application/json' })
      res.end('{}')
    })
    const upstreamPort = String(await listen(upstream))
    closers.push(() => close(upstream))

    // Only the target moves to the stand-in upstream; changeOrigin comes from vite.config.ts.
    const dev = await createServer({
      configFile,
      logLevel: 'silent',
      server: {
        host: '127.0.0.1',
        port: 0,
        hmr: false,
        ws: false,
        proxy: { '/api': { target: `http://127.0.0.1:${upstreamPort}` } },
      },
    })
    await dev.listen()
    closers.push(() => dev.close())
    const devPort = String(portOf(dev.httpServer as Server | null))

    const res = await fetch(`http://127.0.0.1:${devPort}/api/snapshot`)
    expect(res.status).toBe(200)
    expect(seen).toEqual([`127.0.0.1:${upstreamPort}`])
  })
})

describe('build (acceptances 2 and 7)', () => {
  it('sets base "./"', () => {
    expect(config.base).toBe('./')
  })

  it('index.html references scripts and styles as ./assets/<name>-<hash>.<ext>', async () => {
    const result = await build({ configFile, logLevel: 'silent', build: { write: false } })
    const outputs = (Array.isArray(result) ? result : [result]).flatMap((r) =>
      'output' in r ? r.output : [],
    )
    const files = outputs.map((o) => o.fileName)

    const html = outputs.find((o) => o.fileName === 'index.html')
    if (html?.type !== 'asset') throw new Error(`no index.html asset in ${files.join(', ')}`)
    const source =
      typeof html.source === 'string' ? html.source : new TextDecoder().decode(html.source)

    const scripts = [...source.matchAll(/<script[^>]*\ssrc="([^"]+)"/g)].map((m) => m[1] ?? '')
    const styles = [...source.matchAll(/<link[^>]*rel="stylesheet"[^>]*href="([^"]+)"/g)].map(
      (m) => m[1] ?? '',
    )
    expect(scripts.length).toBeGreaterThan(0)
    expect(styles.length).toBeGreaterThan(0)
    for (const ref of [...scripts, ...styles]) {
      expect(ref).toMatch(/^\.\/assets\/[\w-]+-[\w-]{8,}\.(js|css)$/)
      expect(files).toContain(ref.slice(2))
    }
  }, 60_000)
})
