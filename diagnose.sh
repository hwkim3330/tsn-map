#!/bin/bash
# Python/System Diagnostic Script
echo "=== System Diagnostic ==="
echo ""

echo "1. Python versions:"
which python3
python3 --version 2>&1
which pip3
pip3 --version 2>&1

echo ""
echo "2. Python path:"
python3 -c "import sys; print('\n'.join(sys.path))" 2>&1

echo ""
echo "3. Check for broken packages:"
pip3 check 2>&1 | head -20

echo ""
echo "4. Test gTTS install:"
pip3 install --break-system-packages gtts 2>&1 | tail -5
python3 -c "from gtts import gTTS; print('gTTS OK')" 2>&1

echo ""
echo "5. Test venv:"
python3 -m venv /tmp/test_venv 2>&1
if [ -f /tmp/test_venv/bin/activate ]; then
    echo "venv OK"
    rm -rf /tmp/test_venv
else
    echo "venv FAILED"
fi

echo ""
echo "6. ffmpeg:"
ffmpeg -version 2>&1 | head -1

echo ""
echo "7. Disk space:"
df -h /home/kim | tail -1

echo ""
echo "=== Diagnosis Complete ==="
