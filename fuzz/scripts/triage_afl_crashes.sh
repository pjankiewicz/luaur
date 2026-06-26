#!/usr/bin/env bash
set -euo pipefail

export RUSTC_WRAPPER=""

if ! command -v cargo-afl >/dev/null 2>&1; then
  echo "cargo-afl is not installed. Run: cargo install cargo-afl" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FUZZ_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${FUZZ_DIR}"

TARGET="${TARGET:-typeck}"
AFL_OUT="${AFL_OUT:-artifacts/afl/${TARGET}}"
CRASH_DIR="${CRASH_DIR:-${AFL_OUT}/default/crashes}"
TMIN_DIR="${TMIN_DIR:-${AFL_OUT}/tmin}"
TRIAGE_DIR="${TRIAGE_DIR:-${AFL_OUT}/triage}"
BIN_PATH="${BIN_PATH:-target/debug/${TARGET}}"
MAX_CRASHES="${MAX_CRASHES:-0}"
SKIP_TMIN="${SKIP_TMIN:-0}"
REPLAY_CPU_LIMIT_SECS="${REPLAY_CPU_LIMIT_SECS:-5}"

mkdir -p "${TMIN_DIR}" "${TRIAGE_DIR}"

if [[ ! -d "${CRASH_DIR}" ]]; then
  echo "Crash directory does not exist: ${CRASH_DIR}" >&2
  exit 1
fi

extract_signature() {
  local log_path="$1"
  local status="$2"
  local panic_line_no
  panic_line_no="$(grep -n -m1 "panicked at " "${log_path}" | cut -d: -f1 || true)"
  if [[ -n "${panic_line_no}" ]]; then
    local panic_line reason_line panic_site reason_norm
    panic_line="$(sed -n "${panic_line_no}p" "${log_path}" || true)"
    reason_line="$(sed -n "$((panic_line_no + 1))p" "${log_path}" || true)"

    panic_site="$(printf "%s" "${panic_line}" | sed -E 's/.*panicked at ([^:]+:[0-9]+:[0-9]+):.*/\1/')"
    if [[ "${panic_site}" == "${panic_line}" ]]; then
      panic_site="unknown"
    fi

    # Normalize input-specific noise so the same bug buckets together regardless
    # of the concrete numbers / ids in the offending Luau program.
    reason_norm="$(printf "%s" "${reason_line}" | sed -E \
      -e 's/TypeId\([0-9]+\)/TypeId(...)/g' \
      -e 's/TypePackId\([0-9]+\)/TypePackId(...)/g' \
      -e 's/[0-9]+\.[0-9]+/N.N/g' \
      -e 's/[0-9]+/N/g' \
      -e 's/[[:space:]]+/ /g' \
      -e 's/^ //; s/ $//')"

    if [[ -n "${reason_norm}" ]]; then
      printf "panic@%s | %s" "${panic_site}" "${reason_norm}"
      return
    fi
    printf "panic@%s" "${panic_site}"
    return
  fi

  local sig
  sig="$(head -n 1 "${log_path}" || true)"
  if [[ -z "${sig}" ]]; then
    sig="non-panic crash (exit=${status})"
  fi
  printf "%s" "${sig}" | sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//'
}

echo "[triage] building AFL target: ${TARGET}" >&2
cargo afl build --bin "${TARGET}" >/dev/null

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

cache_dir="${tmp_dir}/cache"
mkdir -p "${cache_dir}"
details_tsv="${TRIAGE_DIR}/details.tsv"
summary_tsv="${TRIAGE_DIR}/summary.tsv"
report_txt="${TRIAGE_DIR}/summary.txt"
: > "${details_tsv}"

total_crashes=0
tmin_failures=0
replay_failures=0
processed_crashes=0
live_crashes=0

shopt -s nullglob
crash_files=()
for crash in "${CRASH_DIR}"/id:*; do
  [[ -f "${crash}" ]] || continue
  base="$(basename "${crash}")"
  if [[ "${base}" == "README.txt" ]]; then
    continue
  fi
  crash_files+=("${crash}")
done

if [[ "${MAX_CRASHES}" -gt 0 ]] && [[ "${#crash_files[@]}" -gt "${MAX_CRASHES}" ]]; then
  crash_files=("${crash_files[@]:0:${MAX_CRASHES}}")
fi

total_crashes="${#crash_files[@]}"
if [[ "${total_crashes}" -eq 0 ]]; then
  echo "No crash inputs found under ${CRASH_DIR}" >&2
  exit 1
fi

echo "[triage] crashes to process: ${total_crashes} (skip_tmin=${SKIP_TMIN})" >&2

for idx in "${!crash_files[@]}"; do
  crash="${crash_files[$idx]}"
  base="$(basename "${crash}")"
  progress=$((idx + 1))
  processed_crashes="${progress}"

  if [[ "${progress}" -eq 1 ]] || [[ "${progress}" -eq "${total_crashes}" ]] || (( progress % 10 == 0 )); then
    echo "[triage] processing ${progress}/${total_crashes}: ${base}" >&2
  fi

  min_path="${TMIN_DIR}/${base}"

  if [[ ! -f "${min_path}" ]]; then
    if [[ "${SKIP_TMIN}" == "1" ]]; then
      cp -f "${crash}" "${min_path}"
    else
      echo "[triage] tmin ${progress}/${total_crashes}: ${base}" >&2
      if ! cargo afl tmin -i "${crash}" -o "${min_path}" -- "${BIN_PATH}" >/dev/null 2>&1; then
        cp -f "${crash}" "${min_path}"
        tmin_failures=$((tmin_failures + 1))
      fi
    fi
  fi

  input_hash="$(shasum -a 256 "${min_path}" | awk '{print $1}')"
  cached_sig="${cache_dir}/${input_hash}.sig"
  cached_status="${cache_dir}/${input_hash}.status"
  cached_log="${cache_dir}/${input_hash}.log"

  if [[ ! -f "${cached_sig}" ]]; then
    if bash -c 'ulimit -t "$4"; AFL_FUZZER_LOOPCOUNT=1 LUAUR_FUZZ_STDIN=1 RUST_BACKTRACE=0 "$1" < "$2" > "$3" 2>&1' _ "${BIN_PATH}" "${min_path}" "${cached_log}" "${REPLAY_CPU_LIMIT_SECS}" 2>/dev/null; then
      status=0
    else
      status=$?
    fi

    if [[ ${status} -eq 0 ]]; then
      replay_failures=$((replay_failures + 1))
      continue
    fi

    sig="$(extract_signature "${cached_log}" "${status}")"
    printf "%s" "${sig}" > "${cached_sig}"
    printf "%s" "${status}" > "${cached_status}"
  fi

  status="$(cat "${cached_status}")"
  sig="$(cat "${cached_sig}")"
  if [[ "${status}" -eq 0 ]]; then
    replay_failures=$((replay_failures + 1))
    continue
  fi
  live_crashes=$((live_crashes + 1))
  bucket_id="$(printf "%s" "${sig}" | shasum -a 256 | awk '{print substr($1,1,12)}')"

  printf "%s\t%s\t%s\t%s\t%s\t%s\n" \
    "${bucket_id}" "${status}" "${input_hash}" "${base}" "${min_path}" "${sig}" >> "${details_tsv}"
done

awk -F '\t' '
{
  bucket=$1
  count[bucket]++
  if (!(bucket in status)) {
    status[bucket]=$2
    input_hash[bucket]=$3
    crash_file[bucket]=$4
    min_path[bucket]=$5
    signature[bucket]=$6
  }
}
END {
  for (bucket in count) {
    printf "%08d\t%s\t%s\t%s\t%s\t%s\t%s\n",
      count[bucket], bucket, status[bucket], input_hash[bucket], crash_file[bucket], min_path[bucket], signature[bucket]
  }
}
' "${details_tsv}" | sort -r > "${summary_tsv}"

{
  echo "AFL crash triage report"
  echo "target: ${TARGET}"
  echo "crash_dir: ${CRASH_DIR}"
  echo "max_crashes: ${MAX_CRASHES}"
  echo "skip_tmin: ${SKIP_TMIN}"
  echo "replay_cpu_limit_secs: ${REPLAY_CPU_LIMIT_SECS}"
  echo "total_crash_files: ${total_crashes}"
  echo "processed_crash_files: ${processed_crashes}"
  echo "still_crashing_inputs: ${live_crashes}"
  echo "tmin_failures: ${tmin_failures}"
  echo "replay_non_crashing_inputs: ${replay_failures}"
  echo
  echo "Top buckets (count bucket status sample_input_hash sample_crash_file)"
  awk -F '\t' '
  {
    printf "%s %s %s %s %s\n", $1, $2, $3, $4, $5
    printf "  %s\n", $7
  }' "${summary_tsv}" | sed -E 's/^0+([0-9])/\1/' | head -n 20
} > "${report_txt}"

echo "[triage] wrote ${details_tsv}" >&2
echo "[triage] wrote ${summary_tsv}" >&2
echo "[triage] wrote ${report_txt}" >&2
cat "${report_txt}"
