-- Read-only checks on the fully constructed source tables. Alias strings are
-- recorded independently; no alias-to-index relation is inferred.
local result = {}
for _, reviewed in ipairs(spellStatSetReviewed) do
 local gem = assert(data.gems[reviewed.key])
 local effect = assert(data.skills[reviewed.primary_effect_id])
 assert(gem.grantedEffect == effect and gem.grantedEffectList[1] == effect)
 assert(#gem.grantedEffectList == 1 and #gem.additionalGrantedEffects == 0)
 local keys = {}
 for key in pairs(effect.statSets) do
  assert(type(key) == "number" and key % 1 == 0 and key >= 1 and key <= 64,
   "unexpected constructed stat-set key")
  keys[#keys + 1] = key
 end
 table.sort(keys)
 assert(#keys > 0 and #keys <= 64)
 local inventory = {}
 for ordinal, key in ipairs(keys) do
  assert(key == ordinal, "sparse constructed stat-set inventory")
  local set = effect.statSets[key]
  local flags = {}
  for name, value in pairs(set.baseFlags or {}) do
   assert(type(name) == "string" and #name <= 256 and type(value) == "boolean")
   flags[name] = value
  end
  assert(type(set.label) == "string" and #set.label <= 512)
  assert(type(set.statDescriptionScope) == "string" and #set.statDescriptionScope <= 512)
  inventory[#inventory + 1] = {index = key, label = set.label,
   stat_description_scope = set.statDescriptionScope, base_flags = flags}
 end
 local aliases = {}
 for _, reference in ipairs(reviewed.declared_additional_stat_sets) do
  local alias = gem["additionalStatSet" .. reference.index]
  assert(alias == reference.id)
  local in_effect_list = false
  for _, candidate in ipairs(gem.grantedEffectList) do
   if candidate.id == alias then in_effect_list = true end
  end
  aliases[#aliases + 1] = {metadata_index = reference.index, id = alias,
   standalone_skill = data.skills[alias] ~= nil, in_effect_list = in_effect_list}
 end
 result[#result + 1] = {physical_id = gem.id, effect = effect.id,
  primary_same_object = true, stat_sets = inventory, aliases = aliases,
  has_global_effect = effect.hasGlobalEffect == true, has_parts = effect.parts ~= nil}
end
return result
