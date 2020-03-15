#!/usr/bin/python2
from __future__ import print_function
import sys
from rosgraph_msgs.msg import Log

def read_message():
    """Read the rust ROS message and check the values"""
    name = "Test"
    msg = "This is a test"
    topics = ["Topic1", "Topic2"]

    errors = ""
    file = open("tests/log_rust.bytes", mode="r")

    log = Log()
    log.deserialize(file.read())

    if log.name != name:
        errors += "\nname error: Expected '{}' Got '{}'".format(name, log.name)
    if log.msg != msg:
        errors += "\nmsg error: Expected '{}' Got '{}'".format(msg, log.msg)
    if log.topics != topics:
        errors += "\ntopics error: Expected '{}' Got '{}'".format(msg, log.topics)
    return errors

if __name__ == "__main__":
    errors = read_message()
    if len(errors) == 0:
        sys.exit(0)
    else:
        print(sys.stderr, "Got errors:")
        print(errors)

