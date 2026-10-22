# Read all paginated check-run responses. A trusted latest run must be successful
# for every required name on the exact release commit; stale successes cannot win.
if ($required[0] | type != "array" or length == 0) then
  error("Required check policy must be a nonempty array")
else . end
| [.[].check_runs[]? | select(
    .app.id == 15368 and .head_sha == $sha
    and (.check_suite.id as $id | $suites | index($id) != null)
  )] as $runs
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
