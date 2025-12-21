/**
 * TanStack Start Production Server with Bun
 *
 * Custom server wrapper that properly handles Web Response objects for Bun runtime.
 */

import path from 'node:path'

const SERVER_PORT = Number(process.env.PORT ?? 3000)
const CLIENT_DIRECTORY = './dist/client'
const SERVER_ENTRY_POINT = './dist/server/server.js'

const log = {
  info: (message: string) => console.log(`[INFO] ${message}`),
  success: (message: string) => console.log(`[SUCCESS] ${message}`),
  error: (message: string) => console.log(`[ERROR] ${message}`)
}

interface InMemoryAsset {
  data: Uint8Array
  type: string
  size: number
}

async function initializeStaticRoutes(clientDirectory: string): Promise<Record<string, (req: Request) => Response>> {
  const routes: Record<string, (req: Request) => Response> = {}

  log.info(`Loading static assets from ${clientDirectory}...`)

  try {
    const glob = new Bun.Glob('**/*')
    let fileCount = 0

    for await (const relativePath of glob.scan({ cwd: clientDirectory })) {
      const filepath = path.join(clientDirectory, relativePath)
      const route = `/${relativePath.split(path.sep).join(path.posix.sep)}`

      try {
        const file = Bun.file(filepath)
        if (!(await file.exists()) || file.size === 0) continue

        const bytes = new Uint8Array(await file.arrayBuffer())
        const asset: InMemoryAsset = {
          data: bytes,
          type: file.type || 'application/octet-stream',
          size: bytes.byteLength
        }

        routes[route] = () =>
          new Response(new Uint8Array(asset.data), {
            headers: {
              'Content-Type': asset.type,
              'Content-Length': String(asset.size),
              'Cache-Control': 'public, max-age=31536000, immutable'
            }
          })

        fileCount++
      } catch (error) {
        if (error instanceof Error && error.name !== 'EISDIR') {
          log.error(`Failed to load ${filepath}: ${error.message}`)
        }
      }
    }

    log.success(`Preloaded ${fileCount} static files`)
  } catch (error) {
    log.error(`Failed to load static files: ${String(error)}`)
  }

  return routes
}

async function initializeServer() {
  log.info('Starting TanStack Start Production Server...')

  // Load TanStack Start server handler
  let handler: { fetch: (request: Request) => Response | Promise<Response> }
  try {
    const serverModule = (await import(SERVER_ENTRY_POINT)) as {
      default: { fetch: (request: Request) => Response | Promise<Response> }
    }
    handler = serverModule.default
    log.success('TanStack Start application handler initialized')
  } catch (error) {
    log.error(`Failed to load server handler: ${String(error)}`)
    process.exit(1)
  }

  // Build static routes
  const routes = await initializeStaticRoutes(CLIENT_DIRECTORY)

  // Create Bun server
  const server = Bun.serve({
    port: SERVER_PORT,

    async fetch(req: Request) {
      const url = new URL(req.url)

      // Try static routes first
      const staticHandler = routes[url.pathname]
      if (staticHandler) {
        return staticHandler(req)
      }

      // Fallback to TanStack Start handler
      try {
        const response = await handler.fetch(req)
        // Ensure we return a proper Web Response
        if (response instanceof Response) {
          return response
        }
        // Handle NodeResponse conversion if needed
        const nodeResponse = response as unknown as {
          status?: number
          statusText?: string
          headers?: Headers | Record<string, string>
          body?: ReadableStream | string | null
          text?: () => Promise<string>
        }

        const body = nodeResponse.body || (nodeResponse.text ? await nodeResponse.text() : null)
        return new Response(body, {
          status: nodeResponse.status || 200,
          statusText: nodeResponse.statusText,
          headers: nodeResponse.headers as HeadersInit
        })
      } catch (error) {
        log.error(`Server handler error: ${String(error)}`)
        return new Response('Internal Server Error', { status: 500 })
      }
    },

    error(error) {
      log.error(`Uncaught server error: ${error.message}`)
      return new Response('Internal Server Error', { status: 500 })
    }
  })

  log.success(`Server listening on http://localhost:${server.port}`)
}

initializeServer().catch((error: unknown) => {
  log.error(`Failed to start server: ${String(error)}`)
  process.exit(1)
})
