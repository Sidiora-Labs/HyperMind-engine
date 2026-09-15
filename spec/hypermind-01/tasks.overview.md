## Tasks

Waves are vertical slices. Every wave adds to every layer and closes with an integration journey (task `W.9` or the last task of the wave) that all lane tasks in the wave require, and that the next wave's lane tasks require in turn. Lanes run in parallel within a wave. Root owns `root-foundation` and `integration` tasks and alone edits shared manifests and spec state.

Reading a task: `owner_lane` is the file lane; `requires` are task ids that must be done first; `reqs` are the acceptance-criteria ids this task must satisfy; `touches` is the exclusive path scope; `verify_cmd` is what `cg spec done` runs.
