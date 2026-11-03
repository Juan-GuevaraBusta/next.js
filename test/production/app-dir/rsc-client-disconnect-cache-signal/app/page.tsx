import { Suspense, cacheSignal } from 'react'
import { connection } from 'next/server'

async function Hang({ id }: { id: string }) {
  const signal = cacheSignal()
  console.log(`hang-started ${id}`)
  // Only resolves once React aborts the render, e.g. after a client disconnect.
  await new Promise<void>((resolve) => {
    signal?.addEventListener('abort', () => {
      console.log(`cache-signal-aborted ${id}`)
      resolve()
    })
  })
  return <p>aborted</p>
}

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<{ id: string; streaming?: string }>
}) {
  await connection()
  const { id, streaming } = await searchParams
  if (streaming === '1') {
    return (
      <Suspense fallback={<p>stream-started</p>}>
        <Hang id={id} />
      </Suspense>
    )
  }
  return <Hang id={id} />
}
