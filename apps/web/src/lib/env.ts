import { createEnv } from '@t3-oss/env-core'
import { z } from 'zod'

export const env = createEnv({
  server: {
    DATABASE_URL: z.url(),
    BETTER_AUTH_SECRET: z.string().min(32),
    BETTER_AUTH_URL: z.url(),
    INTERNAL_API_SECRET: z.string(),
    PROXY_URL: z.url(),
    TUNNL_MANAGEMENT_API_URL: z.url(),
    SSH_HOST: z.string(),
    SSH_PORT: z.string()
  },
  runtimeEnv: process.env,
  emptyStringAsUndefined: true
})
