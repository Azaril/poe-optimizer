-- Complete the pinned application item-loading task before importing caller items.
-- A frame is a time slice, not a database-readiness guarantee. Keep the original
-- callback/coroutine so prototype construction, iteration and parser caches agree.
local max_resumes = ...
local app = assert(main, "Missing PoB application during item database initialization")
assert(type(app.onFrameFuncs) == "table", "Missing PoB initialization callbacks")
assert(type(app.uniqueDB) == "table" and type(app.rareDB) == "table",
    "Missing PoB item databases")
local resumes = 0
while app.onFrameFuncs.LoadItems do
    assert(resumes < max_resumes, "PoB item database initialization exceeded callback limit")
    resumes = resumes + 1
    app.onFrameFuncs.LoadItems()
    assert(not launch.promptMsg, launch.promptMsg)
end
assert(not app.uniqueDB.loading and not app.rareDB.loading,
    "PoB item database initialization ended before databases were ready")
