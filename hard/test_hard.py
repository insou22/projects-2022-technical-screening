"""
====================TESTS====================
You may add your own tests if you would like. We will run our much more extensive
hidden tests on your submission.
"""
from hard import is_unlocked

def test_empty():
    assert is_unlocked([], "COMP1511") == True
    assert is_unlocked([], "COMP9301") == False

def test_single():
    assert is_unlocked(["MATH1081"], "COMP3153") == True
    assert is_unlocked(["COMP1511", "COMP1521", "COMP1531"], "COMP3153") == False

def test_compound():
    assert is_unlocked(["MATH1081", "COMP1511"], "COMP2111") == True
    assert is_unlocked(["COMP1521", "COMP2521"], "COMP3151") == True
    assert is_unlocked(["COMP1917", "DPST1092"], "COMP3151") == False

def test_simple_uoc():
    assert is_unlocked(["COMP1511", "COMP1521", "COMP1531", "COMP2521"], "COMP4161") == True
    assert is_unlocked(["COMP1511", "COMP1521"], "COMP4161") == False

def test_hard_uoc():
    assert is_unlocked(["COMP6441", "COMP6443", "COMP1511", "COMP6447"], "COMP9302") == True
    assert is_unlocked(["COMP6841", "COMP6443", "COMP1511", "COMP6449"], "COMP9302") == False

test_empty()
test_single()
test_compound()
test_simple_uoc()
test_hard_uoc()
