from setuptools import Extension, setup
from setuptools.command.build_ext import build_ext


class ExactMathBuildExt(build_ext):
    def build_extensions(self):
        if self.compiler.compiler_type == "msvc":
            flags = ["/fp:strict"]
        elif self.compiler.compiler_type in {"unix", "mingw32", "cygwin"}:
            flags = ["-fno-fast-math", "-ffp-contract=off"]
        else:
            flags = []
        for extension in self.extensions:
            extension.extra_compile_args.extend(flags)
        super().build_extensions()


setup(
    cmdclass={"build_ext": ExactMathBuildExt},
    ext_modules=[
        Extension("bokehforge._scoring_native", ["src/bokehforge/_scoring_native.c"], optional=True),
        Extension("bokehforge._alignment_native", ["src/bokehforge/_alignment_native.c"], optional=True),
    ],
)
