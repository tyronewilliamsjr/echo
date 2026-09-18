import { useForm } from '@tanstack/react-form'
import { createFileRoute } from '@tanstack/react-router'
import { useMutation } from '@tanstack/react-query'

type PrivacySettings = {
  retain_audio: boolean
  retain_transcript: boolean
  allow_reminder: boolean
  allow_embedding: boolean
}

export const Route = createFileRoute('/_authed/dashboard/settings/')({
  component: RouteComponent,
  loader: async () => {
    const response = await fetch('/api/users/settings/privacy', {
      credentials: 'include',
    })
    return response.json() as Promise<PrivacySettings>
  },
})

function PrivacySettingsForm({
  initialValues,
}: {
  initialValues: PrivacySettings
}) {
  const form = useForm({
    defaultValues: {
      retain_audio: initialValues.retain_audio,
      retain_transcript: initialValues.retain_transcript,
      allow_reminder: initialValues.allow_reminder,
      allow_embedding: initialValues.allow_embedding,
    },
    validators: {
      onSubmit: ({ value, formApi }) => {
        // console.log(value)
      },
    },
    onSubmit: async ({ value, formApi }) => {
      await mutateAsync(value)
    },
  })

  const { mutateAsync } = useMutation({
    mutationFn: async (values: PrivacySettings) => {
      const response = await fetch('/api/users/settings/privacy', {
        method: 'POST',
        body: JSON.stringify(values),
        credentials: 'include',
        headers: {
          'Content-Type': 'application/json',
        },
      })
      return response.json()
    },
    mutationKey: ['settings', 'privacy'],
  })

  return (
    <div>
      <h1>Privacy Settings</h1>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <form.Field name="retain_audio">
          {(field) => {
            return (
              <label>
                <input
                  type="checkbox"
                  name="retain_audio"
                  checked={field.state.value}
                  onChange={(e) => field.handleChange(e.target.checked)}
                />
                Retain Audio
              </label>
            )
          }}
        </form.Field>

        <form.Field name="retain_transcript">
          {(field) => {
            return (
              <label>
                <input
                  type="checkbox"
                  name="retain_transcript"
                  checked={field.state.value}
                  onChange={(e) => field.handleChange(e.target.checked)}
                />
                Retain Transcript
              </label>
            )
          }}
        </form.Field>

        <form.Field name="allow_reminder">
          {(field) => {
            return (
              <label>
                <input
                  type="checkbox"
                  name="allow_reminder"
                  checked={field.state.value}
                  onChange={(e) => field.handleChange(e.target.checked)}
                />
                Allow Reminder
              </label>
            )
          }}
        </form.Field>

        <form.Field name="allow_embedding">
          {(field) => {
            return (
              <label>
                <input
                  type="checkbox"
                  name="allow_embedding"
                  checked={field.state.value}
                  onChange={(e) => field.handleChange(e.target.checked)}
                />
                Allow Embedding
              </label>
            )
          }}
        </form.Field>

        <button type="submit">Save</button>
      </form>
    </div>
  )
}

function RouteComponent() {
  const data = Route.useLoaderData()
  console.log(data)
  return (
    <div>
      <PrivacySettingsForm initialValues={data} />
    </div>
  )
}
