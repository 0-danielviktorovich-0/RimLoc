"""CLI: python3 -m testlab.ui_automation --config testlab/ui_automation/examples/rimloc-acceptance.yaml"""

from __future__ import annotations

import argparse
import sys
from datetime import datetime

from . import journey


def main() -> int:
    parser = argparse.ArgumentParser(
        prog='testlab.ui_automation',
        description='Background (off-screen) UI journeys for Tauri/WKWebView apps')
    parser.add_argument('--config', required=True,
                        help='path to a journey YAML config')
    parser.add_argument('--shots-dir', default=None,
                        help='override config shots_dir')
    args = parser.parse_args()

    if args.shots_dir:
        import yaml
        with open(args.config, encoding='utf-8') as f:
            cfg = yaml.safe_load(f)
        cfg['shots_dir'] = args.shots_dir
        import tempfile
        tmp = tempfile.NamedTemporaryFile('w', suffix='.yaml', delete=False)
        yaml.safe_dump(cfg, tmp, allow_unicode=True)
        tmp.close()
        config_path = tmp.name
    else:
        config_path = args.config

    stamp = datetime.now().strftime('%H:%M:%S')

    def log(msg: str) -> None:
        print(f'[{stamp}] {msg}', flush=True)

    report = journey.run(config_path, log=log)
    print(report.summary())
    return 0 if report.ok else 1


if __name__ == '__main__':
    sys.exit(main())
