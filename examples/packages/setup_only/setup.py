import os
import re
from setuptools import setup, find_packages


# NOTE: If updating requirements make sure to also check Pipfile for any locks
# NOTE: When updating botocore make sure to update awscli/boto3 versions below
install_requires = [
    # pegged to also match items in `extras_require`
    'botocore>=1.23.24,<1.23.25',
    'aiohttp>=3.3.1',
    'wrapt>=1.10.10',
    'aioitertools>=0.5.1',
]

extras_require = {
    'awscli': ['awscli>=1.22.24,<1.22.25'],
    'boto3': ['boto3>=1.20.24,<1.20.25'],
}

hello = "hello"
hello += " world"
print(hello)

setup(
    name='awdawdawdwad',
    version=read_version(),
    description='Async client for aws services using botocore and aiohttp',
    long_description='\n\n'.join((read('README.rst'), read('CHANGES.rst'))),
    long_description_content_type='text/x-rst',
    classifiers=classifiers,
    author="lorem ipsum author",
    author_email="lorem ipsum",
    url='lorem ipsum',
    download_url='lorem ipsum',
    license='Apache 2',
    packages=find_packages(),
    python_requires='>=3.6',
    install_requires=install_requires,
    extras_require=extras_require,
    include_package_data=True
)
