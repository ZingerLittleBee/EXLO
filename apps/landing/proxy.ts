import { NextResponse } from 'next/server'
import type { NextRequest } from 'next/server'

const DOCS_ORIGIN = 'https://exlo-docs.vercel.app'

export function middleware(request: NextRequest) {
  const { pathname, search } = request.nextUrl

  if (!pathname.startsWith('/docs')) {
    return NextResponse.next()
  }

  const upstreamPath = pathname.replace(/^\/docs/, '') || '/'
  const url = new URL(`${upstreamPath}${search}`, DOCS_ORIGIN)

  return NextResponse.rewrite(url)
}

export const config = {
  matcher: ['/docs/:path*']
}
