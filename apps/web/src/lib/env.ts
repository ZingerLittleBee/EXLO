import { createEnv } from '@t3-oss/env-core'
import { z } from 'zod'

export const env = createEnv({
  server: {
    DATABASE_URL: z.url(),
    AUTH_SECRET: z.string().min(32),
    HOMEPAGE_URL: z.url(),
    INTERNAL_API_SECRET: z.string(),
    TUNNL_MANAGEMENT_API_URL: z.url(),
    SSH_HOST: z.string(),
    SSH_PORT: z.string(),
    TUNNEL_URL: z.string()
  },
  runtimeEnv: process.env,
  emptyStringAsUndefined: true
})
