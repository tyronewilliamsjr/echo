import { queryOptions } from '@tanstack/react-query'

export const sessionQueryOptions = queryOptions({
  queryKey: ['session'],
  queryFn: async () => {
    const res = await fetch('/api/auth/session', {
      credentials: 'include',
    })

    if (res.ok) return res.json() as Promise<{ id: number; username: string }>

    return null
  },
  staleTime: 1000 * 60 * 5,
})
