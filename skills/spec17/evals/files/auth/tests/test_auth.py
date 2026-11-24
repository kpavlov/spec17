from src.auth import active

def test_login():
    """[AUTH-07]"""
    assert active(0)
