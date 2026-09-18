import { useState } from 'react'
import { Link, useRouter } from '@tanstack/react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import { Button } from '@/components/ui/button'
import { sessionQueryOptions } from '@/util/session'

export function UserButton() {
  const [open, setOpen] = useState(false)
  const queryClient = useQueryClient()
  const router = useRouter()
  const { mutate } = useMutation({
    mutationKey: ['logout'],
    mutationFn: async () => {
      const res = await fetch('/api/auth/logout', {
        credentials: 'include',
        method: 'POST',
      })
    },
    onSuccess: () => {
      queryClient.setQueryData(sessionQueryOptions.queryKey, null)
      router.navigate({ to: '/' })
    },
  })

  const { data } = useQuery(sessionQueryOptions)

  if (data) {
    return (
      <div>
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger>
            <div>{data.username}</div>
          </PopoverTrigger>

          <PopoverContent>
            <div>{data.username}</div>
            <div>
              <Link
                className="w-full justify-start"
                to="/dashboard/settings"
                onClick={() => setOpen(false)}
              >
                Settings
              </Link>
            </div>

            <div>
              <button
                className="text-red-500"
                type="button"
                aria-label="Logout"
                onClick={() => {
                  mutate()
                  setOpen(false)
                }}
              >
                Logout
              </button>
            </div>
          </PopoverContent>
        </Popover>
      </div>
    )
  }

  return (
    <div>
      <div>testing</div>
    </div>
  )
}
