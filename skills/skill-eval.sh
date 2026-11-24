### https://docs.nvidia.com/skills/skillevaluator/

export SKILL_EVAL_LLM_PROVIDER=openai

. "$(dirname "$0")/../.env" || exit 1
export OPENAI_API_KEY

rm -rf reports

skillevaluator tier1 ./spec17 --no-llm -o reports/tier1

skillevaluator harness-check ./spec17 > reports/harness-check.txt
skillevaluator tier2 ./spec17 -r cli,json,html -o reports/tier2-one

skillevaluator create-eval-dataset ./spec17 --full --refine --prompt eval-guidance.md

skillevaluator tier3 validate ./spec17 --strict
