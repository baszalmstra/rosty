#!/usr/bin/python

import rospy
from rosgraph_msgs.msg import Log


def generate_message():
    """Generate a python log message to use for decoding later"""
    file = open("log.bytes", mode="w")
    log = Log(name="Test", level=1, msg="This is a test", topics=["Topic1", "Topic2"])
    log.serialize(file)


if __name__ == "__main__":
    generate_message()
