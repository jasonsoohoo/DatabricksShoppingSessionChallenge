param([ValidateSet('build','fixtures','dev','small','practice','final','compare')][string]$Action = 'dev', [ValidatePattern('^[a-zA-Z0-9-]+$')][string]$Run = 'default', [string]$Image = 'shopping-sessions:latest')
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$root = (Resolve-Path "$PSScriptRoot/..").Path
Push-Location $root
try {
    if ($Action -eq 'build') {
        docker buildx build --platform linux/amd64 --load -t shopping-sessions:latest .
        docker buildx build --platform linux/amd64 --load -t shopping-validator -f scripts/Test.Dockerfile .
        return
    }
    $suffix = if ($Run -eq "default") { "" } else { "-$Run" }
    $limits = @('--rm','--platform','linux/amd64','--network','none','--read-only','--user','1000:1000',
        '--cpus','8','--memory','24g','--memory-swap','24g','--pids-limit','4096','--cap-drop','ALL',
        '--security-opt','no-new-privileges','--tmpfs','/tmp:rw,size=64m,mode=1777','-v','/scratch',
        '-e','HOME=/scratch','-e','TMPDIR=/scratch','-e','SCRATCH_DIR=/scratch','-e','NCPU=8','-e','MEM_GB=24')
    if ($Action -eq 'fixtures') {
        docker run @limits -v "${root}/fixtures:/fixtures:ro" -v "${root}/scripts:/checks:ro" shopping-validator python /checks/run_fixtures.py /fixtures -- /app/run.sh
    } elseif ($Action -eq 'compare') {
        docker run @limits -v "${root}/scripts:/checks:ro" -v "${root}/.local/out-dev${suffix}:/actual:ro" -v "${root}/dev-answer:/expected:ro" shopping-validator python /checks/compare.py /actual /expected
    } else {
        $folder = New-Item -Force -ItemType Directory ".local/out-$Action$suffix"
        $source = if ($Action -in @("practice","final")) { "shopping-buildathon-$Action" } else { "${root}/.local/${Action}" }
        "checkout=$(git rev-parse HEAD)"
        docker image inspect $Image --format 'image={{.Id}} bytes={{.Size}}'
        $watch = [Diagnostics.Stopwatch]::StartNew()
        docker run @limits -v "${source}:/data:ro" -v "${folder}:/output" $Image
        "container wall {0:N3}s" -f $watch.Elapsed.TotalSeconds
    }
} finally { Pop-Location }
