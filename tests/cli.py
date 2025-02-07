from utils import cargo_run, goto_root, mk_and_cd_tmp_dir

# NOTE: raising a cli error must be idempotent
def assert_cli_error(args: list[str]):
    assert "cli error" in cargo_run(args, check=False, stderr=True)
    assert cargo_run(args, check=False) != 0

def cli():
    goto_root()
    mk_and_cd_tmp_dir()
    cargo_run(["init"])

    assert_cli_error(["invalid-command"])
    assert_cli_error(["config", "--invalid-flag"])

    # in git, args and flags can be interleaved
    # so is in ragit
    assert_cli_error(["add", "--invalid-flag"])
    assert_cli_error(["add", "--invalid-flag", "valid-file-name"])
    assert_cli_error(["add", "valid-file-name", "--invalid-flag"])

    # TODO: more tests...
