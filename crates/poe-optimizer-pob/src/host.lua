-- Installed after upstream headless definitions, before application initialization.
GetTime = _optimizer_time
GetScriptPath = function() return _optimizer_user_path end
GetRuntimePath = function() return _optimizer_runtime_path end
GetUserPath = function() return _optimizer_user_path end
GetWorkDir = function() return _optimizer_source_path end
MakeDir = _optimizer_make_dir
SetWorkDir = function() error("Changing the evaluator working directory is unsupported") end
RemoveDir = function() error("Directory removal is disabled in the evaluator") end
print = _optimizer_log
ConPrintf = function(fmt, ...) _optimizer_log(string.format(fmt, ...)) end
ConPrintTable = function() end
LaunchSubScript = function() error("Background scripts/network access are disabled in the evaluator") end
io.read = function() error((launch and launch.promptMsg) or "Interactive input is disabled in the evaluator") end
io.popen = function() error("Child shell processes are disabled in the evaluator") end
local open = io.open
io.open = function(path, mode)
    mode = mode or "r"
    if mode:find("[wa+]") then _optimizer_check_write(path) end
    return open(path, mode)
end
io.output = function() error("Changing default output is disabled in the evaluator") end
io.write = function(...) _optimizer_log(...) end
os.execute = function() error("Shell execution is disabled in the evaluator") end
os.remove = function() error("File removal is disabled in the evaluator") end
os.rename = function() error("File rename is disabled in the evaluator") end
os.exit = function() error("Upstream attempted to exit the evaluator") end
local getenv = os.getenv
os.getenv = function(name)
    if name == "REGENERATE_MOD_CACHE" or name == "CI" then return nil end
    return getenv(name)
end
