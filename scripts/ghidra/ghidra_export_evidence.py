#!/usr/bin/env python
#@category SwitchRecomp
#@runtime Jython

"""
Export deterministic Ghidra analysis evidence as JSON.

Usage (headless):
  -postScript ghidra_export_evidence.py /abs/path/to/ghidra-evidence.json
"""

import json
import time

from ghidra.program.util import DefinedDataIterator


def collect_functions(program):
    manager = program.getFunctionManager()
    iterator = manager.getFunctions(True)
    items = []
    while iterator.hasNext():
        func = iterator.next()
        items.append(
            {
                "name": func.getName(),
                "entry": str(func.getEntryPoint()),
                "size": int(func.getBody().getNumAddresses()),
            }
        )
    items.sort(key=lambda item: item["entry"])
    return items


def collect_imports(program):
    symbol_table = program.getSymbolTable()
    iterator = symbol_table.getExternalSymbols()
    names = set()
    while iterator.hasNext():
        symbol = iterator.next()
        names.add(symbol.getName())
    return sorted(list(names))


def collect_strings(program, limit):
    strings = []
    for data in DefinedDataIterator.definedStrings(program):
        value = data.getValue()
        if value is None:
            continue
        strings.append(
            {
                "address": str(data.getAddress()),
                "value": str(value)[:256],
            }
        )
        if len(strings) >= limit:
            break
    strings.sort(key=lambda item: item["address"])
    return strings


def write_json(path, payload):
    with open(path, "w") as handle:
        handle.write(json.dumps(payload, indent=2, sort_keys=True))
        handle.write("\n")


def main():
    args = list(getScriptArgs())
    if len(args) < 1:
        raise RuntimeError("output path argument is required")

    output_path = args[0]
    program = currentProgram

    payload = {
        "schema_version": "1",
        "generated_unix": int(time.time()),
        "program_name": program.getName(),
        "language_id": str(program.getLanguageID()),
        "compiler_spec_id": str(program.getCompilerSpec().getCompilerSpecID()),
        "functions": collect_functions(program),
        "imports": collect_imports(program),
        "strings": collect_strings(program, 2000),
        # Reserved fields to align with pipeline expectations.
        "cfg_edges": [],
        "unresolved_indirect_branches": [],
    }

    write_json(output_path, payload)
    print("wrote ghidra evidence to {}".format(output_path))


main()
