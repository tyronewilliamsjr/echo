import { createFileRoute, Outlet, redirect } from '@tanstack/react-router'
import { sessionQueryOptions } from '@/util/session'

export const Route = createFileRoute('/_authed')({
  ssr: false,
  beforeLoad: async ({ context }) => {
    console.log('grabbing session')
    const session =
      await context.queryClient.ensureQueryData(sessionQueryOptions)

    if (!session) {
      throw redirect({
        to: '/auth',
      })
    }

    return {
      session,
    }
  },

  component: AuthenticatedLayout,
})

function AuthenticatedLayout() {
  return <Outlet />
}
