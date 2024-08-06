import timeit
import json
from jsonschema.protocols import Validator
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012

from outlines.fsm.json_schema import to_regex


def setup_benchmark():
    schema = {
        "type": "integer",
    }

    Validator.check_schema(schema)

    # Build reference resolver
    schema_resource = Resource(contents=schema, specification=DRAFT202012)
    uri = schema_resource.id() if schema_resource.id() is not None else ""
    registry = Registry().with_resource(uri=uri, resource=schema_resource)
    resolver = registry.resolver()

    content = schema_resource.contents

    return resolver, content


def run_to_regex():
    resolver, content = setup_benchmark()
    return to_regex(resolver, content)


def run_benchmark():
    # Run the benchmark
    number_of_runs = 100000
    time_taken = timeit.timeit(run_to_regex, number=number_of_runs)

    # Calculate and print results
    average_time = time_taken / number_of_runs
    print(f"Average time: {average_time:.6f} seconds")
    print(f"Average time: {average_time * 1e6:.3f} microseconds")
    print(f"Average time: {average_time * 1e9:.3f} nanoseconds")


def main():
    schema = {
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                 "minLength": 2, "maxLength": 5
            },
        },
    }

    # schema = {
    #     "type": "object",
    #     "properties": {
    #         "name": {"type": "string"},
    #         "flag": {"type": "boolean"},
    #     },
    # }

    Validator.check_schema(schema)

    # Build reference resolver
    schema_resource = Resource(contents=schema, specification=DRAFT202012)
    uri = schema_resource.id() if schema_resource.id() is not None else ""
    registry = Registry().with_resource(uri=uri, resource=schema_resource)
    resolver = registry.resolver()

    content = schema_resource.contents
    res = to_regex(resolver, content)

    print(len(res))
    print(res)


if __name__ == "__main__":
    main()
