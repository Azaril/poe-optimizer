#!/usr/bin/env python3
"""Strict finite tree export for the offline owned catalog compiler.

Reads pinned JSON and validates source file hashes. Never executes Lua, imports
builds, interprets stat text, allocates owned IDs, or certifies game legality.
"""
from __future__ import annotations
import argparse
from collections import defaultdict
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re

@dataclass(frozen=True)
class Limits:
    input_bytes: int = 8 * 1024 * 1024
    total_source_bytes: int = 16 * 1024 * 1024
    output_bytes: int = 8 * 1024 * 1024
    rows: int = 10000
    entries: int = 200000
    string_bytes: int = 16384
HARD = Limits()
ROOT_FIELDS = set("assets classes connectionArt constants ddsCoords groups jewelSlots max_x max_y min_x min_y nodeOverlay nodes tree".split())
CLASS_FIELDS = set("ascendancies background base_dex base_int base_str integerId name".split())
ASC_FIELDS = set("background id internalId name replace replaceBy".split())
NODE_FIELDS = set("activeEffectImage aliasPassiveSocket applyToArmour ascendancyName classesStart connectionArt connections containJewelSocket flavourText group icon isAscendancyStart isAttribute isFreeAllocate isJewelSocket isKeystone isMultipleChoice isMultipleChoiceOption isNotable isOnlyImage isSwitchable name noRadius nodeOverlay options orbit orbitIndex recipe sinister skill stats stringId unlockConstraint".split())
OPTION_FIELDS = set("ascendancyName icon id name nodeOverlay stats".split())
FLAGS = set("applyToArmour containJewelSocket isAscendancyStart isAttribute isFreeAllocate isJewelSocket isKeystone isMultipleChoice isMultipleChoiceOption isNotable isOnlyImage isSwitchable noRadius sinister".split())

def sha(data): return hashlib.sha256(data).hexdigest()
def compact(value): return json.dumps(value,ensure_ascii=False,allow_nan=False,separators=(",",":")).encode()
def pretty(value): return (json.dumps(value,ensure_ascii=False,allow_nan=False,indent=2)+"\n").encode()

def bounded_pretty(value,maximum):
    """Preserve pretty bytes, stopping before retaining an oversized document."""
    encoded=bytearray()
    encoder=json.JSONEncoder(ensure_ascii=False,allow_nan=False,indent=2)
    for chunk in encoder.iterencode(value):
        data=chunk.encode("utf-8")
        if len(data)>maximum-len(encoded): raise ValueError("output byte bound")
        encoded.extend(data)
    if len(encoded)>=maximum: raise ValueError("output byte bound")
    encoded.append(10)
    return bytes(encoded)
def unique(pairs):
    out = {}
    for k,v in pairs:
        if k in out: raise ValueError("duplicate JSON key: " + k)
        out[k] = v
    return out

def decode(data):
    def nonfinite(value): raise ValueError("nonfinite JSON: " + value)
    return json.loads(data,object_pairs_hook=unique,parse_constant=nonfinite)
def read(path,maximum):
    with path.open("rb") as stream: data=stream.read(maximum+1)
    if len(data)>maximum: raise ValueError("input byte bound: " + str(path))
    return data

def fields(value,allowed,required,label):
    if not isinstance(value,dict) or not required <= value.keys() or value.keys()-allowed:
        raise ValueError("unreviewed fields: " + label)
def text(value,limits):
    if not isinstance(value,str) or len(value.encode())>limits.string_bytes: raise ValueError("bounded text required")
    return value
def node_key(value):
    if type(value) is not int or not 0 < value <= 2**32-1: raise ValueError("positive integer node key required")
    return str(value)
def strings(value,limits):
    if not isinstance(value,list) or len(value)>limits.entries: raise ValueError("bounded string list required")
    return [text(v,limits) for v in value]

def produce(manifest_bytes,source_root,limits=HARD):
    for name,maximum in vars(HARD).items():
        value=getattr(limits,name)
        if type(value) is not int or not 0<value<=maximum: raise ValueError("invalid limit: "+name)
    if len(manifest_bytes)>limits.input_bytes: raise ValueError("manifest byte bound")
    manifest=decode(manifest_bytes)
    fields(manifest,set("schema_version tree_version tree_file source attribute_lanes".split()),set("schema_version tree_version tree_file source attribute_lanes".split()),"manifest")
    if manifest["schema_version"]!=1: raise ValueError("unsupported exporter manifest")
    version=text(manifest["tree_version"],limits)
    if not version: raise ValueError("empty tree version")
    source=manifest["source"]
    fields(source,{"system","revision","files"},{"system","revision","files"},"source")
    if source["system"]!="path_of_building2" or not re.fullmatch(r"[0-9a-f]{40}",source["revision"]): raise ValueError("unreviewed source context")
    if not isinstance(source["files"],list) or not 1<=len(source["files"])<=32: raise ValueError("source file bound")
    root=source_root.resolve(); pins={}; remaining=limits.total_source_bytes
    for pin in source["files"]:
        fields(pin,{"path","sha256"},{"path","sha256"},"source pin")
        name=text(pin["path"],limits); path=(root/name).resolve()
        if not path.is_relative_to(root) or name in pins or "\\" in name or not re.fullmatch(r"[0-9a-f]{64}",pin["sha256"]): raise ValueError("source path or hash")
        data=read(path,min(limits.input_bytes,remaining));remaining-=len(data)
        data=data.replace(b"\r\n",b"\n")
        if sha(data)!=pin["sha256"]: raise ValueError("source pin differs: "+name)
        pins[name]=data
    if manifest["tree_file"] not in pins: raise ValueError("tree file is not pinned")
    tree=decode(pins[manifest["tree_file"]]);fields(tree,ROOT_FIELDS,ROOT_FIELDS,"tree")
    raw=tree["nodes"]
    if not isinstance(raw,dict) or not 1<=len(raw)<=limits.rows: raise ValueError("tree row bound")
    budget=limits.entries
    def charge(n):
        nonlocal budget
        budget-=n
        if budget<0: raise ValueError("aggregate tree entry bound")
    nodes={}; string_ids=set()
    for key,n in raw.items():
        fields(n,NODE_FIELDS,set("skill stringId name stats connections group orbit orbitIndex icon".split()),"node")
        if key!=node_key(n["skill"]): raise ValueError("node key mismatch")
        if text(n["stringId"],limits) in string_ids: raise ValueError("duplicate stringId")
        string_ids.add(n["stringId"]);text(n["name"],limits);strings(n["stats"],limits)
        for flag in FLAGS & n.keys():
            if type(n[flag]) is not bool: raise ValueError("nonboolean semantic flag")
        if not isinstance(n["connections"],list): raise ValueError("connection list")
        charge(1+len(n["stats"])+len(n["connections"]))
        if "classesStart" in n: strings(n["classesStart"],limits)
        if "ascendancyName" in n: text(n["ascendancyName"],limits)
        options=n.get("options",{})
        if not isinstance(options,(dict,list)): raise ValueError("option shape")
        if isinstance(options,list) != bool(n.get("isAttribute")): raise ValueError("attribute option shape")
        if options and not n.get("isAttribute") and not (n.get("isSwitchable") or n.get("isAscendancyStart")): raise ValueError("unclassified options")
        charge(len(options))
        for option in options.values() if isinstance(options,dict) else options:
            fields(option,OPTION_FIELDS,set(),"option")
            if "stats" in option: strings(option["stats"],limits);charge(len(option["stats"]))
            for k in ["name","ascendancyName"]:
                if k in option:text(option[k],limits)
        unlock=n.get("unlockConstraint",{"nodes":[]})
        fields(unlock,{"nodes","ascendancy"},{"nodes"},"unlock")
        if not isinstance(unlock["nodes"],list): raise ValueError("unlock list")
        for u in unlock["nodes"]: node_key(u)
        charge(len(unlock["nodes"]))
        nodes[key]=n
    edges=set(); unresolved=set(); adjacency=defaultdict(set); visual_edges=0; self_edges=[]
    for key,n in nodes.items():
        for c in n["connections"]:
            fields(c,{"id","orbit"},{"id","orbit"},"connection");other=node_key(c["id"])
            if other not in nodes: unresolved.add((key,other));continue
            if key==other:self_edges.append(key);continue
            if n.get("isOnlyImage") or nodes[other].get("isOnlyImage"):visual_edges+=1;continue
            edge=tuple(sorted((key,other),key=int));edges.add(edge);adjacency[key].add(other);adjacency[other].add(key)
    parents={}
    for key,n in nodes.items():
        if n.get("isMultipleChoiceOption"):
            candidates=[p for p in adjacency[key] if nodes[p].get("isMultipleChoice")]
            if len(candidates)!=1: raise ValueError("attached option needs one parent")
            parents[key]=candidates[0]
    roots={}; asc_roots={}
    def insert_root(index,name,key):
        if name in index and index[name]!=key: raise ValueError("ambiguous root")
        index[name]=key
    for key,n in nodes.items():
        for name in n.get("classesStart",[]):insert_root(roots,name,key)
        if n.get("isAscendancyStart"):
            insert_root(asc_roots,n["ascendancyName"],key)
            for name in n.get("options",{}):insert_root(asc_roots,name,key)
    if not isinstance(tree["classes"],list) or not tree["classes"]: raise ValueError("class list")
    charge(len(tree["classes"])); classes=[];seen_classes=set();seen_asc=set()
    for c in tree["classes"]:
        fields(c,CLASS_FIELDS,CLASS_FIELDS,"class");key=node_key(c["integerId"])
        if key in seen_classes or c["name"] not in roots: raise ValueError("class/root identity")
        seen_classes.add(key); ascend=[]
        if not isinstance(c["ascendancies"],list): raise ValueError("ascendancy list")
        charge(len(c["ascendancies"]))
        for i,a in enumerate(c["ascendancies"],1):
            fields(a,ASC_FIELDS,{"background","id","internalId","name"},"ascendancy")
            for k in a.keys()-{"background"}:text(a[k],limits)
            if a["internalId"] in seen_asc or a["name"] not in asc_roots or i>65535: raise ValueError("ascendancy/root identity")
            seen_asc.add(a["internalId"]);ascend.append({"key":a["internalId"],"ordinal":i,"root":asc_roots[a["name"]]})
        classes.append({"key":key,"root":roots[c["name"]],"ascendancies":ascend})
    lanes=manifest["attribute_lanes"]
    if not isinstance(lanes,list) or not 1<=len(lanes)<=32: raise ValueError("attribute lane bound")
    lane_names=set(); lane_fields=set()
    for lane in lanes:
        fields(lane,{"attribute","option_name"},{"attribute","option_name"},"lane")
        for k in lane:text(lane[k],limits)
        if lane["option_name"] in lane_names or lane["attribute"] in lane_fields: raise ValueError("duplicate attribute lane")
        lane_names.add(lane["option_name"]);lane_fields.add(lane["attribute"])
    attributes=None; output_nodes=[]; deferred=[]
    for key,n in sorted(nodes.items(),key=lambda row:int(row[0])):
        pool="ascendancy" if n.get("ascendancyName") else "ordinary"
        if n.get("classesStart") or n.get("isAscendancyStart"):kind={"kind":"implicit_root"}
        elif n.get("isOnlyImage"):kind={"kind":"unsupported","value":{"code":"presentation-only-node"}}
        elif key in parents:kind={"kind":"attached_choice","value":{"parent":parents[key]}}
        elif n.get("isAttribute"):
            kind={"kind":"attribute","value":{"pool":pool}}
            options={o["name"]:o["stats"] for o in n["options"]}
            if len(options)!=len(n["options"]) or set(options)!=lane_names: raise ValueError("attribute options differ from manifest")
            actual=[{"key":lane["attribute"],"stats":options[lane["option_name"]]} for lane in lanes]
            if attributes is not None and actual!=attributes: raise ValueError("attribute parents have different option semantics")
            attributes=actual
        else:kind={"kind":"allocation","value":{"pool":pool}}
        views=[]
        if isinstance(n.get("options"),dict):
            for name,o in sorted(n["options"].items()):
                stats=o.get("stats",n["stats"])
                # Inherited lists are serialized once per view, even though the
                # source stores them once. Charge expansion before adding a view.
                charge(len(stats))
                views.append({"selector":name,"stats":stats})
        output_nodes.append({"key":key,"kind":kind,"stats":n["stats"],"views":views,"unlock":[node_key(u) for u in n.get("unlockConstraint",{}).get("nodes",[])]})
        if n.get("unlockConstraint") or n.get("options") or any(n.get(k) for k in ["isFreeAllocate","aliasPassiveSocket","containJewelSocket","noRadius","sinister","applyToArmour"]):
            deferred.append({"node":key,"fields":sorted(k for k in ["unlockConstraint","options","isFreeAllocate","aliasPassiveSocket","containJewelSocket","noRadius","sinister","applyToArmour"] if n.get(k))})
    if attributes is None: attributes=[]
    catalog={"schema_version":1,"tree_version":version,"source":source,"classes":sorted(classes,key=lambda c:int(c["key"])),"nodes":output_nodes,"edges":[{"left":a,"right":b} for a,b in sorted(edges,key=lambda e:tuple(map(int,e)))],"unresolved_edges":[{"left":a,"right":b} for a,b in sorted(unresolved,key=lambda e:tuple(map(int,e)))],"attribute_options":attributes}
    outputs={"tree-catalog.json":bounded_pretty(catalog,limits.output_bytes)}
    outputs["source-facts.json"]=bounded_pretty({"schema_version":1,"source":source,"manifest_sha256":sha(manifest_bytes),"catalog_sha256":sha(outputs["tree-catalog.json"]),"nodes":len(nodes),"edges":len(edges),"unresolved_edges":len(unresolved),"visual_connections_excluded":visual_edges,"self_connections_excluded":sorted(self_edges,key=int),"unconverted_node_fields":deferred,"source_execution":False,"numerical_rules_compiled":False,"legality_established":False},limits.output_bytes-len(outputs["tree-catalog.json"]))
    return outputs

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest",type=Path,required=True);parser.add_argument("--source-root",type=Path,required=True)
    dest=parser.add_mutually_exclusive_group(required=True);dest.add_argument("--output-dir",type=Path);dest.add_argument("--check-dir",type=Path)
    args=parser.parse_args(); outputs=produce(read(args.manifest,HARD.input_bytes),args.source_root)
    if args.check_dir:
        for name,data in outputs.items():
            if read(args.check_dir/name,HARD.output_bytes)!=data: raise ValueError("persisted tree export differs: "+name)
        print("verified finite owned tree export; no source execution")
    else:
        args.output_dir.mkdir(parents=True,exist_ok=False)
        for name,data in outputs.items():
            with (args.output_dir/name).open("xb") as stream:stream.write(data)
        print("wrote finite tree inputs; Rust validation and publication required")
if __name__=="__main__":
    try:main()
    except (ValueError,KeyError,TypeError,OSError,RecursionError) as error:raise SystemExit("owned tree export: "+str(error)) from error
