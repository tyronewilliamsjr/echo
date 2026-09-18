import { Dialog } from '#/components/ui/dialog'
import { createFileRoute, useRouter } from '@tanstack/react-router'
import { useForm } from '@tanstack/react-form'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { sessionQueryOptions } from '#/util/session'

export const Route = createFileRoute('/auth/')({
  component: RouteComponent,
})

function SigninForm() {
  const queryClient = useQueryClient()
  const router = useRouter()
  const form = useForm({
    defaultValues: {
      email: '',
      password: '',
    },
    async onSubmit({ value, formApi }) {
      const result = await mutateAsync(value)
      router.navigate({ to: '/dashboard' })

      // if (result.error) {
      //   switch (result.error.code) {
      //     case 'INVALID_EMAIL':
      //       formApi.setFieldMeta('email', (meta) => ({
      //         ...meta,
      //         errorMap: {
      //           ...meta.errorMap,
      //           onSubmit: 'Invalid email',
      //         },
      //       }))
      //       break
      //     case 'INVALID_EMAIL_OR_PASSWORD':
      //       formApi.setFieldMeta('email', (meta) => ({
      //         ...meta,
      //         errorMap: {
      //           ...meta.errorMap,
      //           onSubmit: 'Invalid email or password',
      //         },
      //       }))

      //       break
      //   }
      // }
    },
    validators: {
      onSubmit: ({ value }) => {
        if (!value.email) {
          return {
            fields: {
              email: 'Email is required',
            },
          }
        }
      },
    },
  })

  const { mutate, mutateAsync } = useMutation({
    mutationKey: ['signin', 'email'],
    mutationFn: async ({
      email,
      password,
    }: {
      email: string
      password: string
    }) => {
      const response = await fetch('/api/auth/signin', {
        body: JSON.stringify({ email, password }),
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (response.ok)
        return response.json() as Promise<{ id: number; username: string }>

      if (response.status === 401) {
        form.setFieldMeta('email', (meta) => ({
          ...meta,
          errorMap: {
            ...meta.errorMap,
            onSubmit: 'Invalid email or password',
          },
        }))
      }
    },
    onSuccess: (data) => {
      queryClient.setQueryData(sessionQueryOptions.queryKey, data)
    },
  })

  return (
    <Dialog>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <span className="text-2xl font-bold mb-4">Sign In</span>

        <form.Field name="email">
          {(field) => {
            console.log(field.state.meta)
            return (
              <div className="">
                <span className="block mb-1 text-slate-500">Email</span>
                <input
                  type="text"
                  name={field.name}
                  value={field.state.value}
                  placeholder="Email"
                  onChange={(e) => field.handleChange(e.target.value)}
                />

                {field.state.meta.errors.map((error) => (
                  <p key={error as string}>{error}</p>
                ))}
              </div>
            )
          }}
        </form.Field>

        <form.Field name="password">
          {(field) => {
            return (
              <div className="">
                <span className="block mb-1 text-slate-500">Password</span>
                <input
                  type="password"
                  name={field.name}
                  value={field.state.value}
                  placeholder="Password"
                  onChange={(e) => field.handleChange(e.target.value)}
                />

                {field.state.meta.errors.map((error) => (
                  <p key={error as string}>{error}</p>
                ))}
              </div>
            )
          }}
        </form.Field>

        <button type="submit">Submit</button>
      </form>
    </Dialog>
  )
}

function SignupForm() {
  const router = useRouter()
  const queryClient = useQueryClient()

  const form = useForm({
    defaultValues: {
      email: '',
      password: '',
    },
    validators: {
      onSubmit: ({ value }) => {
        if (!value.email) {
          return {
            fields: {
              email: 'Email is required',
            },
          }
        } else if (!value.password) {
          return {
            fields: {
              password: 'Password is required',
            },
          }
        }
      },
    },
    onSubmit: async ({ value, formApi }) => {
      try {
        await signupMutation.mutateAsync(value)
        router.navigate({ to: '/dashboard' })
      } catch (error) {
        formApi.setFieldMeta('email', (meta) => ({
          ...meta,
          errorMap: {
            ...meta.errorMap,
            onSubmit: 'Invalid email',
          },
        }))
      }
    },
  })

  const signupMutation = useMutation({
    mutationFn: async ({
      email,
      password,
    }: {
      email: string
      password: string
    }) => {
      const result = await fetch('/api/auth/signup', {
        body: JSON.stringify({ email, password }),
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (result.ok)
        return result.json() as Promise<{ id: number; username: string }>

      throw new Error('Failed to signup')
    },
    onSuccess: (data) => {
      queryClient.setQueryData(sessionQueryOptions.queryKey, data)
    },
    mutationKey: ['signup', 'email'],
  })

  return (
    <Dialog>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          console.log('onsumbit reached passing to react form')
          form.handleSubmit()
        }}
      >
        <h2 className="">Signup</h2>
        <form.Field name="email">
          {(field) => {
            return (
              <label className="block">
                <span className="">Email</span>
                <input
                  type="text"
                  name="email"
                  id="email"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  placeholder="Email ..."
                />

                {field.state.meta.errors.map((error) => (
                  <p key={error as string}>{error}</p>
                ))}
              </label>
            )
          }}
        </form.Field>

        <form.Field name="password">
          {(field) => {
            return (
              <label>
                <span className="">Password</span>
                <input
                  type="password"
                  name="password"
                  id="password"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  placeholder="Password ..."
                />

                {field.state.meta.errors.map((error) => (
                  <p key={error as string}>{error}</p>
                ))}
              </label>
            )
          }}
        </form.Field>

        <button type="submit">Signup</button>
      </form>
    </Dialog>
  )
}

function RouteComponent() {
  return (
    <div className="flex flex-row flex-nowrap justify-center items-center h-full w-full">
      <div className="">
        <div className="">
          <h2>Signin or Signup</h2>
        </div>

        <div></div>

        <div className="">Or</div>

        <div className="">
          <label htmlFor="email" className="block mb-2">
            <span className="block mb-1 text-slate-500">Email</span>
            <input type="text" name="email" id="email" placeholder="Email" />
          </label>

          <div className="flex flex-row justify-center w-full">
            <button type="button" className="rounded bg-sky-500 px-4 py-2">
              Continue
            </button>
          </div>
        </div>

        {/* <SignupForm /> */}
        <SigninForm />
      </div>
    </div>
  )
}
