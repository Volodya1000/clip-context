# Read all paginated check-run responses. A trusted latest run must be successful
# for every required name on the exact release commit; stale successes cannot win.
[.[].check_runs[]? | select(.app.id == 15368 and .head_sha == $sha)] as $runs
| [
    $required[0][] as $name
    | ($runs | map(select(.name == $name)) | max_by(.id)) as $run
    | select($run == null or $run.status != "completed" or $run.conclusion != "success")
    | {
        name: $name,
        status: ($run.status // "missing"),
        conclusion: ($run.conclusion // "none")
      }
  ]
