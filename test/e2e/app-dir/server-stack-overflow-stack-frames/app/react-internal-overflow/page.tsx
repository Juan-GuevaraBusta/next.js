const DEPTH = 100_000

export default function Page() {
  let tree: React.ReactNode = 'leaf'

  for (let i = 0; i < DEPTH; i++) {
    tree = <div>{tree}</div>
  }

  return tree
}
