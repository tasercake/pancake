#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <math.h>
#include <stdint.h>

static int
get_buffer(PyObject *object, Py_buffer *view, int rank, char format, size_t size,
           int writeable)
{
    int flags = PyBUF_STRIDES | PyBUF_FORMAT | (writeable ? PyBUF_WRITABLE : 0);
    if (PyObject_GetBuffer(object, view, flags) < 0) {
        return -1;
    }
    if (view->ndim != rank || view->itemsize != (Py_ssize_t)size ||
        view->format[0] != format || view->format[1] != '\0' ||
        (uintptr_t)view->buf % size != 0) {
        PyErr_SetString(PyExc_ValueError, "Expected an aligned native floating-point buffer");
        return -1;
    }
    for (int axis = 0; axis < rank; axis++) {
        if (view->strides[axis] % (Py_ssize_t)size != 0) {
            PyErr_SetString(PyExc_ValueError, "Expected aligned floating-point strides");
            return -1;
        }
    }
    return 0;
}

static Py_ssize_t
nearest_index(double index, Py_ssize_t length)
{
    if (index < 0.0) {
        return 0;
    }
    if (index >= length) {
        return length - 1;
    }
    return (Py_ssize_t)index;
}

static PyObject *
sample_linear(PyObject *self, PyObject *args)
{
    PyObject *image, *points, *output;
    Py_buffer image_view = {0}, point_view = {0}, output_view = {0};
    PyObject *result = NULL;
    int rgb;
    if (!PyArg_ParseTuple(args, "OOOp:sample_linear", &image, &points, &output, &rgb)) {
        return NULL;
    }
    if (get_buffer(image, &image_view, 3, 'f', sizeof(float), 0) < 0 ||
        get_buffer(points, &point_view, 2, 'd', sizeof(double), 0) < 0 ||
        get_buffer(output, &output_view, 2, 'f', sizeof(float), 1) < 0) {
        goto done;
    }

    int y_axis = rgb ? 0 : 1, x_axis = rgb ? 1 : 2, channel_axis = rgb ? 2 : 0;
    Py_ssize_t height = image_view.shape[y_axis], width = image_view.shape[x_axis];
    Py_ssize_t count = point_view.shape[1];
    int sample_axis = rgb ? 0 : 1, output_channel_axis = 1 - sample_axis;
    if (height < 1 || width < 1 || image_view.shape[channel_axis] != 3 ||
        point_view.shape[0] != 2 || output_view.shape[sample_axis] != count ||
        output_view.shape[output_channel_axis] != 3) {
        PyErr_SetString(PyExc_ValueError, "Alignment sample buffer shapes do not match");
        goto done;
    }

    const char *pixels = image_view.buf, *coordinates = point_view.buf;
    char *samples = output_view.buf;
    Py_ssize_t row_stride = image_view.strides[y_axis], col_stride = image_view.strides[x_axis];
    Py_ssize_t channel_stride = image_view.strides[channel_axis];
    Py_ssize_t point_stride = point_view.strides[1];
    Py_ssize_t sample_stride = output_view.strides[sample_axis];
    Py_ssize_t output_channel_stride = output_view.strides[output_channel_axis];
    int finite = 1;
    Py_BEGIN_ALLOW_THREADS
    for (Py_ssize_t i = 0; i < count; i++) {
        const char *point = coordinates + i * point_stride;
        double y = *(const double *)point;
        double x = *(const double *)(point + point_view.strides[0]);
        if (!isfinite(y) || !isfinite(x)) {
            finite = 0;
            break;
        }
        double top = floor(y), left = floor(x);
        Py_ssize_t y0 = nearest_index(top, height) * row_stride;
        Py_ssize_t x0 = nearest_index(left, width) * col_stride;
        const char *upper_left = pixels + y0 + x0;
        char *sample = samples + i * sample_stride;
        if (y == top && x == left) {
            for (int c = 0; c < 3; c++) {
                float value = *(const float *)(upper_left + c * channel_stride);
                // Interpolation accumulates from positive zero even on grid points.
                *(float *)(sample + c * output_channel_stride) = 0.0f + value;
            }
            continue;
        }

        Py_ssize_t y1 = nearest_index(top + 1.0, height) * row_stride;
        Py_ssize_t x1 = nearest_index(left + 1.0, width) * col_stride;
        const char *upper_right = pixels + y0 + x1;
        const char *lower_left = pixels + y1 + x0;
        const char *lower_right = pixels + y1 + x1;
        double wy0 = 1.0 - (y - top), wy1 = 1.0 - wy0;
        double wx0 = 1.0 - (x - left), wx1 = 1.0 - wx0;
        for (int c = 0; c < 3; c++) {
            Py_ssize_t offset = c * channel_stride;
            // Match SciPy order-1 interpolation. Multiply each pixel by
            // the row and column weights in double precision before summing.
            double value = 0.0;
            value += (*(const float *)(upper_left + offset) * wy0) * wx0;
            value += (*(const float *)(upper_right + offset) * wy0) * wx1;
            value += (*(const float *)(lower_left + offset) * wy1) * wx0;
            value += (*(const float *)(lower_right + offset) * wy1) * wx1;
            *(float *)(sample + c * output_channel_stride) = (float)value;
        }
    }
    Py_END_ALLOW_THREADS
    if (!finite) {
        PyErr_SetString(PyExc_ValueError, "Alignment sample coordinates must be finite");
        goto done;
    }
    result = Py_NewRef(Py_None);

done:
    PyBuffer_Release(&output_view);
    PyBuffer_Release(&point_view);
    PyBuffer_Release(&image_view);
    return result;
}

static PyMethodDef methods[] = {
    {"sample_linear", sample_linear, METH_VARARGS, NULL},
    {NULL, NULL, 0, NULL},
};

static struct PyModuleDef module = {
    PyModuleDef_HEAD_INIT,
    "_alignment_native",
    NULL,
    -1,
    methods,
};

PyMODINIT_FUNC
PyInit__alignment_native(void)
{
    return PyModule_Create(&module);
}
