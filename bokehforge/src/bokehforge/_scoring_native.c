#define PY_SSIZE_T_CLEAN
#include <Python.h>
#include <float.h>
#include <math.h>
#include <stdint.h>
#include <string.h>

static int
buffer_has_type(const Py_buffer *view, char expected, Py_ssize_t itemsize)
{
    const char *format = view->format;
    if (format == NULL || view->itemsize != itemsize || (uintptr_t)view->buf % itemsize) {
        return 0;
    }
    if (format[0] == '@' || format[0] == '=') {
        format++;
    }
    // NumPy's intp uses the platform signed integer code: 'l' on LP64
    // and generally 'q' on Windows. The item size must match Py_ssize_t.
    return format[0] && !format[1] &&
           (format[0] == expected || (expected == 'n' && strchr("ilq", format[0])));
}

static int
get_buffer(PyObject *obj, Py_buffer *view, int ndim, char format, Py_ssize_t itemsize, int writable)
{
    int flags = PyBUF_C_CONTIGUOUS | PyBUF_FORMAT | (writable ? PyBUF_WRITABLE : 0);
    if (PyObject_GetBuffer(obj, view, flags) < 0) {
        return -1;
    }
    if (view->ndim != ndim || (format && !buffer_has_type(view, format, itemsize))) {
        PyErr_SetString(PyExc_ValueError, "Expected an aligned native buffer with the requested shape and dtype");
        return -1;
    }
    return 0;
}

static int
get_rgb_buffer(PyObject *obj, Py_buffer *view)
{
    if (PyObject_GetBuffer(obj, view, PyBUF_STRIDES | PyBUF_FORMAT) < 0) {
        return -1;
    }
    if (view->ndim != 3 || !buffer_has_type(view, 'f', sizeof(float)) ||
        view->shape[2] != 3 || view->strides[2] != sizeof(float) ||
        (view->shape[1] > 1 && view->strides[1] != 3 * sizeof(float)) ||
        (view->shape[0] > 1 && (view->strides[0] % (Py_ssize_t)sizeof(float) ||
                               view->strides[0] < view->shape[1] * (Py_ssize_t)(3 * sizeof(float))))) {
        PyErr_SetString(PyExc_ValueError, "Expected an aligned row-packed RGB buffer");
        return -1;
    }
    return 0;
}

static inline Py_ssize_t
edge_at(const char *edges, Py_ssize_t index)
{
    Py_ssize_t edge;
    memcpy(&edge, edges + index * sizeof(edge), sizeof(edge));
    return edge;
}

static int
valid_edges(const char *edges, Py_ssize_t count, Py_ssize_t end)
{
    if (count < 2 || edge_at(edges, 0) != 0 || edge_at(edges, count - 1) != end) {
        return 0;
    }
    for (Py_ssize_t i = 1; i < count; i++) {
        if (edge_at(edges, i) <= edge_at(edges, i - 1)) {
            return 0;
        }
    }
    return 1;
}

static PyObject *
tile_integral(PyObject *self, PyObject *args)
{
    PyObject *image, *support, *ys, *xs, *output;
    float affected_at;
    Py_ssize_t affected = 0;
    Py_buffer image_view = {0}, support_view = {0}, y_view = {0}, x_view = {0}, output_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OOOOOf:tile_integral", &image, &support, &ys, &xs, &output, &affected_at)) {
        return NULL;
    }
    if (get_buffer(image, &image_view, 2, 0, 0, 0) < 0 ||
        (support != Py_None && get_buffer(support, &support_view, 2, '?', 1, 0) < 0) ||
        get_buffer(ys, &y_view, 1, 'n', sizeof(Py_ssize_t), 0) < 0 ||
        get_buffer(xs, &x_view, 1, 'n', sizeof(Py_ssize_t), 0) < 0 ||
        get_buffer(output, &output_view, 2, 'd', sizeof(double), 1) < 0) {
        goto done;
    }

    Py_ssize_t height = image_view.shape[0], width = image_view.shape[1];
    Py_ssize_t ny = y_view.shape[0], nx = x_view.shape[0];
    const char *y_edges = y_view.buf, *x_edges = x_view.buf;
    int coverage_only = support == Py_None && buffer_has_type(&image_view, '?', 1);
    int float_image = buffer_has_type(&image_view, 'f', sizeof(float));
    if ((!float_image && !coverage_only) || output_view.shape[0] != ny || output_view.shape[1] != nx ||
        (support_view.buf && (support_view.shape[0] != height || support_view.shape[1] != width))) {
        PyErr_SetString(PyExc_ValueError, "Tile integral buffer shapes do not match");
        goto done;
    }
    if (!valid_edges(y_edges, ny, height) || !valid_edges(x_edges, nx, width)) {
        result = Py_NewRef(Py_None);
        goto done;
    }
    double *prefix = PyMem_Calloc(width, sizeof(double));
    if (prefix == NULL) {
        PyErr_NoMemory();
        goto done;
    }

    const uint8_t *row = image_view.buf;
    const uint8_t *mask = support_view.buf;
    double *sample = output_view.buf;
    Py_BEGIN_ALLOW_THREADS
    memset(sample, 0, nx * sizeof(double));
    // Update each column in its original top-to-bottom order. Read input
    // sequentially and retain just one row of double-precision running sums.
    Py_ssize_t y = 0;
    for (Py_ssize_t i = 1; i < ny; i++) {
        Py_ssize_t y_stop = edge_at(y_edges, i);
        for (; y < y_stop; y++, row += width * image_view.itemsize) {
            if (coverage_only) {
                for (Py_ssize_t x = 0; x < width; x++) {
                    prefix[x] += row[x] != 0;
                }
            } else if (mask) {
                const float *values = (const float *)row;
                for (Py_ssize_t x = 0; x < width; x++) {
                    int covered = mask[x] != 0;
                    prefix[x] += (double)values[x] * covered;
                    affected += (values[x] >= affected_at) & covered;
                }
                mask += width;
            } else {
                const float *values = (const float *)row;
                for (Py_ssize_t x = 0; x < width; x++) {
                    prefix[x] += values[x];
                    affected += values[x] >= affected_at;
                }
            }
        }
        // Horizontal sums are only needed on tile-edge rows. Advance in
        // order across the whole row, including between sampled columns.
        sample += nx;
        sample[0] = 0.0;
        double total = prefix[0];
        Py_ssize_t x = 1;
        for (Py_ssize_t j = 1; j < nx; j++) {
            Py_ssize_t x_stop = edge_at(x_edges, j);
            for (; x < x_stop; x++) {
                total += prefix[x];
            }
            sample[j] = total;
        }
    }
    Py_END_ALLOW_THREADS
    PyMem_Free(prefix);
    result = PyLong_FromSsize_t(affected);

done:
    PyBuffer_Release(&output_view);
    PyBuffer_Release(&x_view);
    PyBuffer_Release(&y_view);
    PyBuffer_Release(&support_view);
    PyBuffer_Release(&image_view);
    return result;
}

static PyObject *
rgb_square_difference(PyObject *self, PyObject *args)
{
    PyObject *candidate, *target, *output;
    Py_buffer candidate_view = {0}, target_view = {0}, output_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OOO:rgb_square_difference", &candidate, &target, &output)) {
        return NULL;
    }
    if (get_rgb_buffer(candidate, &candidate_view) < 0 || get_rgb_buffer(target, &target_view) < 0 ||
        get_buffer(output, &output_view, 3, 'd', sizeof(double), 1) < 0) {
        goto done;
    }
    Py_ssize_t height = target_view.shape[0], width = target_view.shape[1];
    if (candidate_view.shape[0] != height || candidate_view.shape[1] != width ||
        output_view.shape[0] != 3 || output_view.shape[1] != height || output_view.shape[2] != width) {
        PyErr_SetString(PyExc_ValueError, "RGB difference buffer shapes do not match");
        goto done;
    }

    const char *c_base = candidate_view.buf, *t_base = target_view.buf;
    Py_ssize_t length = height * width;
    double *red = output_view.buf, *green = red + length, *blue = green + length;
    Py_BEGIN_ALLOW_THREADS
    for (Py_ssize_t y = 0; y < height; y++, red += width, green += width, blue += width) {
        const float *c = (const float *)(c_base + y * candidate_view.strides[0]);
        const float *t = (const float *)(t_base + y * target_view.strides[0]);
        for (Py_ssize_t x = 0; x < width; x++, c += 3, t += 3) {
            double dr = (double)c[0] - (double)t[0];
            double dg = (double)c[1] - (double)t[1];
            double db = (double)c[2] - (double)t[2];
            red[x] = dr * dr;
            green[x] = dg * dg;
            blue[x] = db * db;
        }
    }
    Py_END_ALLOW_THREADS
    result = Py_None;
    Py_INCREF(result);

done:
    PyBuffer_Release(&output_view);
    PyBuffer_Release(&target_view);
    PyBuffer_Release(&candidate_view);
    return result;
}

static PyObject *
rgb_means(PyObject *self, PyObject *args)
{
    PyObject *image, *support;
    Py_buffer image_view = {0}, support_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OO:rgb_means", &image, &support)) {
        return NULL;
    }
    if (get_rgb_buffer(image, &image_view) < 0 ||
        (support != Py_None && get_buffer(support, &support_view, 2, '?', 1, 0) < 0)) {
        goto done;
    }

    Py_ssize_t height = image_view.shape[0], width = image_view.shape[1];
    if (support_view.buf && (support_view.shape[0] != height || support_view.shape[1] != width)) {
        PyErr_SetString(PyExc_ValueError, "RGB mean buffer shapes do not match");
        goto done;
    }
    const char *base = image_view.buf;
    const uint8_t *mask = support_view.buf;
    Py_ssize_t covered = 0;
    double red = 0.0, green = 0.0, blue = 0.0;
    Py_BEGIN_ALLOW_THREADS
    // For a row-major HWC image, NumPy's reduction over H and W is a
    // left-to-right double sum per channel.
    for (Py_ssize_t y = 0; y < height; y++) {
        const float *pixel = (const float *)(base + y * image_view.strides[0]);
        if (mask) {
            for (Py_ssize_t x = 0; x < width; x++, pixel += 3) {
                if (mask[x]) {
                    red += pixel[0];
                    green += pixel[1];
                    blue += pixel[2];
                    covered++;
                }
            }
            mask += width;
        } else {
            for (Py_ssize_t x = 0; x < width; x++, pixel += 3) {
                red += pixel[0];
                green += pixel[1];
                blue += pixel[2];
            }
            covered += width;
        }
    }
    Py_END_ALLOW_THREADS
    if (covered) {
        result = Py_BuildValue("(ddd)", red / covered, green / covered, blue / covered);
    } else {
        // Let NumPy issue its normal empty-slice warning.
        result = Py_None;
        Py_INCREF(result);
    }

done:
    PyBuffer_Release(&support_view);
    PyBuffer_Release(&image_view);
    return result;
}

static inline float
display_sample(float value)
{
    return value < 0.0f ? 0.0f : (value > 1.0f ? 1.0f : value);
}

static PyObject *
lab_xyz(PyObject *self, PyObject *args)
{
    PyObject *rgb, *weights, *xyz;
    Py_ssize_t start;
    Py_buffer rgb_view = {0}, weights_view = {0}, xyz_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OOnO:lab_xyz", &rgb, &weights, &start, &xyz)) {
        return NULL;
    }
    if (get_rgb_buffer(rgb, &rgb_view) < 0 ||
        get_buffer(weights, &weights_view, 2, 'f', sizeof(float), 0) < 0 ||
        get_buffer(xyz, &xyz_view, 2, 'f', sizeof(float), 1) < 0) {
        goto done;
    }

    Py_ssize_t length = xyz_view.shape[1];
    Py_ssize_t available = rgb_view.len / (3 * sizeof(float));
    if (weights_view.shape[0] != 3 || weights_view.shape[1] != 3 || xyz_view.shape[0] != 3 ||
        start < 0 || length == 0 || start > available - length) {
        PyErr_SetString(PyExc_ValueError, "XYZ buffer shapes do not match");
        goto done;
    }
    const char *pixels = rgb_view.buf;
    Py_ssize_t width = rgb_view.shape[1], stride = rgb_view.strides[0];
    int packed = rgb_view.shape[0] == 1 || stride == width * (Py_ssize_t)(3 * sizeof(float));
    const float *m = weights_view.buf;
    float *x = xyz_view.buf, *y = x + length, *z = y + length;

    Py_BEGIN_ALLOW_THREADS
    for (Py_ssize_t i = 0; i < length;) {
        Py_ssize_t source = start + i;
        Py_ssize_t end = length;
        const float *pixel;
        if (packed) {
            pixel = (const float *)pixels + 3 * source;
        } else {
            Py_ssize_t col = source % width;
            pixel = (const float *)(pixels + (source / width) * stride) + 3 * col;
            if (width - col < length - i) {
                end = i + width - col;
            }
        }
        for (; i < end; i++, pixel += 3) {
            float r = display_sample(pixel[0]), g = display_sample(pixel[1]), b = display_sample(pixel[2]);
            // Separate float32 products and sums in the same order as NumPy.
            x[i] = (r * m[0] + g * m[1]) + b * m[2];
            y[i] = (r * m[3] + g * m[4]) + b * m[5];
            z[i] = (r * m[6] + g * m[7]) + b * m[8];
        }
    }
    Py_END_ALLOW_THREADS
    result = Py_NewRef(Py_None);

done:
    PyBuffer_Release(&xyz_view);
    PyBuffer_Release(&weights_view);
    PyBuffer_Release(&rgb_view);
    return result;
}

static inline float
lab_curve(float xyz, float cube)
{
    const double transition = 6.0 / 29.0;
    const float threshold = (float)(transition * transition * transition);
    return xyz > threshold ? cube : xyz * (float)(841.0 / 108.0) + (float)(4.0 / 29.0);
}

static PyObject *
lab_planes(PyObject *self, PyObject *args)
{
    PyObject *xyz, *cubes, *output;
    Py_buffer xyz_view = {0}, cubes_view = {0}, output_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OOO:lab_planes", &xyz, &cubes, &output)) {
        return NULL;
    }
    if (get_buffer(xyz, &xyz_view, 3, 'f', sizeof(float), 0) < 0 ||
        get_buffer(cubes, &cubes_view, 3, 'f', sizeof(float), 0) < 0 ||
        PyObject_GetBuffer(output, &output_view, PyBUF_STRIDES | PyBUF_FORMAT | PyBUF_WRITABLE) < 0) {
        goto done;
    }

    Py_ssize_t images = xyz_view.shape[0], length = xyz_view.shape[2];
    if ((images != 1 && images != 2) || xyz_view.shape[1] != 3 ||
        cubes_view.shape[0] != images || cubes_view.shape[1] != 3 || cubes_view.shape[2] != length ||
        output_view.ndim != 2 || !buffer_has_type(&output_view, 'f', sizeof(float)) ||
        output_view.shape[0] != (images == 1 ? 3 : 5) || output_view.shape[1] != length ||
        output_view.strides[1] != sizeof(float) || output_view.strides[0] < length * (Py_ssize_t)sizeof(float) ||
        output_view.strides[0] % sizeof(float)) {
        PyErr_SetString(PyExc_ValueError, "Lab buffer shapes do not match");
        goto done;
    }

    const float *x = xyz_view.buf, *y = x + length, *z = y + length;
    const float *cx = cubes_view.buf, *cy = cx + length, *cz = cy + length;
    Py_ssize_t step = output_view.strides[0] / sizeof(float);
    float *first = output_view.buf, *second = first + step, *third = second + step;
    Py_BEGIN_ALLOW_THREADS
    if (images == 1) {
        for (Py_ssize_t i = 0; i < length; i++) {
            float lx = lab_curve(x[i], cx[i]), ly = lab_curve(y[i], cy[i]), lz = lab_curve(z[i], cz[i]);
            first[i] = 116.0f * ly - 16.0f;
            second[i] = 500.0f * (lx - ly);
            third[i] = 200.0f * (ly - lz);
        }
    } else {
        const float *tx = z + length, *ty = tx + length, *tz = ty + length;
        const float *tcx = cz + length, *tcy = tcx + length, *tcz = tcy + length;
        float *delta_a = third + step, *delta_b = delta_a + step;
        for (Py_ssize_t i = 0; i < length; i++) {
            float lx = lab_curve(x[i], cx[i]), ly = lab_curve(y[i], cy[i]), lz = lab_curve(z[i], cz[i]);
            float tlx = lab_curve(tx[i], tcx[i]), tly = lab_curve(ty[i], tcy[i]), tlz = lab_curve(tz[i], tcz[i]);
            float cl = 116.0f * ly - 16.0f, tl = 116.0f * tly - 16.0f;
            first[i] = cl;
            second[i] = tl;
            third[i] = cl - tl;
            delta_a[i] = 500.0f * (lx - ly) - 500.0f * (tlx - tly);
            delta_b[i] = 200.0f * (ly - lz) - 200.0f * (tly - tlz);
        }
    }
    Py_END_ALLOW_THREADS
    result = Py_NewRef(Py_None);

done:
    PyBuffer_Release(&output_view);
    PyBuffer_Release(&cubes_view);
    PyBuffer_Release(&xyz_view);
    return result;
}

static int
same_plane(const Py_buffer *view, Py_ssize_t height, Py_ssize_t width)
{
    return view->shape[0] == height && view->shape[1] == width;
}

static inline void
contrast_pairs(const float *c_first, const float *c_second,
               const float *t_first, const float *t_second,
               const uint8_t *mask_first, const uint8_t *mask_second,
               float *lost_first, float *lost_second, float *extra_first, float *extra_second,
               Py_ssize_t count, double spacing)
{
    for (Py_ssize_t x = 0; x < count; x++) {
        if (mask_first && !(mask_first[x] && mask_second[x])) {
            continue;
        }

        // The NumPy orientation first subtracts in float32, then divides
        // by the float64 direction length. Keep that rounding boundary.
        const double cg = (float)(c_second[x] - c_first[x]) / spacing;
        const double tg = (float)(t_second[x] - t_first[x]) / spacing;
        const double c = fabs(cg), t = fabs(tg);
        const double earned = cg * tg > 0.0 ? (c < t ? c : t) : 0.0;
        const double loss = t - earned, gain = c - earned;
        const float lost = loss / (loss + 5.0), extra = gain / (gain + 5.0);
        lost_first[x] = lost > lost_first[x] ? lost : lost_first[x];
        lost_second[x] = lost > lost_second[x] ? lost : lost_second[x];
        extra_first[x] = extra > extra_first[x] ? extra : extra_first[x];
        extra_second[x] = extra > extra_second[x] ? extra : extra_second[x];
    }
}

static PyObject *
contrast_damage(PyObject *self, PyObject *args)
{
    PyObject *candidate, *target, *support, *lost, *extra;
    double diagonal;
    Py_buffer c_view = {0}, t_view = {0}, support_view = {0}, lost_view = {0}, extra_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OOOOOd:contrast_damage",
                          &candidate, &target, &support, &lost, &extra, &diagonal)) {
        return NULL;
    }
    if (get_buffer(candidate, &c_view, 2, 'f', sizeof(float), 0) < 0 ||
        get_buffer(target, &t_view, 2, 'f', sizeof(float), 0) < 0 ||
        (support != Py_None && get_buffer(support, &support_view, 2, '?', 1, 0) < 0) ||
        get_buffer(lost, &lost_view, 2, 'f', sizeof(float), 1) < 0 ||
        get_buffer(extra, &extra_view, 2, 'f', sizeof(float), 1) < 0) {
        goto done;
    }

    const Py_ssize_t height = c_view.shape[0], width = c_view.shape[1];
    if (!same_plane(&t_view, height, width) || !same_plane(&lost_view, height, width) ||
        !same_plane(&extra_view, height, width) ||
        (support != Py_None && !same_plane(&support_view, height, width))) {
        PyErr_SetString(PyExc_ValueError, "Contrast damage buffer shapes do not match");
        goto done;
    }

    const float *c = c_view.buf, *t = t_view.buf;
    const uint8_t *mask = support_view.buf;
    float *loss = lost_view.buf, *gain = extra_view.buf;
    Py_BEGIN_ALLOW_THREADS
    for (Py_ssize_t y = 0; y < height && width; y++) {
        const uint8_t *right = mask ? mask + 1 : NULL;
        contrast_pairs(c, c + 1, t, t + 1, mask, right,
                       loss, loss + 1, gain, gain + 1, width - 1, 1.0);
        if (y + 1 < height) {
            const uint8_t *below = mask ? mask + width : NULL;
            const uint8_t *below_right = below ? below + 1 : NULL;
            contrast_pairs(c, c + width, t, t + width, mask, below,
                           loss, loss + width, gain, gain + width, width, 1.0);
            contrast_pairs(c, c + width + 1, t, t + width + 1, mask, below_right,
                           loss, loss + width + 1, gain, gain + width + 1, width - 1, diagonal);
            contrast_pairs(c + width, c + 1, t + width, t + 1, below, right,
                           loss + width, loss + 1, gain + width, gain + 1, width - 1, diagonal);
        }
        c += width;
        t += width;
        loss += width;
        gain += width;
        if (mask) {
            mask += width;
        }
    }
    Py_END_ALLOW_THREADS
    result = Py_None;
    Py_INCREF(result);

done:
    PyBuffer_Release(&extra_view);
    PyBuffer_Release(&lost_view);
    PyBuffer_Release(&support_view);
    PyBuffer_Release(&t_view);
    PyBuffer_Release(&c_view);
    return result;
}

static inline double
clip_unit(double value)
{
    return value < 0.0 ? 0.0 : value > 1.0 ? 1.0 : value;
}

static inline double
channel_range_damage(double c, double t)
{
    const double c_display = clip_unit(c), t_display = clip_unit(t);
    const double headroom = fabs((c - c_display) - (t - t_display));
    const double distance = c_display < 1.0 - c_display ? c_display : 1.0 - c_display;
    const double endpoint = 1.0 - clip_unit(distance / 0.012);
    const double clipping = fabs(c_display - t_display) * endpoint;
    const double error = headroom > clipping ? headroom : clipping;
    return error / (error + 0.075);
}

static PyObject *
range_damage(PyObject *self, PyObject *args)
{
    PyObject *candidate, *target, *output;
    Py_buffer c_view = {0}, t_view = {0}, output_view = {0};
    PyObject *result = NULL;
    if (!PyArg_ParseTuple(args, "OOO:range_damage", &candidate, &target, &output)) {
        return NULL;
    }
    if (get_buffer(candidate, &c_view, 3, 'f', sizeof(float), 0) < 0 ||
        get_buffer(target, &t_view, 3, 'f', sizeof(float), 0) < 0 ||
        get_buffer(output, &output_view, 2, 'f', sizeof(float), 1) < 0) {
        goto done;
    }

    const Py_ssize_t height = c_view.shape[0], width = c_view.shape[1];
    if (c_view.shape[2] != 3 || t_view.shape[2] != 3 ||
        !same_plane(&t_view, height, width) || !same_plane(&output_view, height, width)) {
        PyErr_SetString(PyExc_ValueError, "Range damage buffer shapes do not match");
        goto done;
    }

    const float *c = c_view.buf, *t = t_view.buf;
    float *range = output_view.buf;
    const Py_ssize_t count = c_view.len / (3 * sizeof(float));
    Py_BEGIN_ALLOW_THREADS
    // Rounding the maximum to float32 selects the same result as rounding
    // the running maximum after each channel in the NumPy implementation.
    for (Py_ssize_t i = 0; i < count; i++, c += 3, t += 3) {
        const double red = channel_range_damage(c[0], t[0]);
        const double green = channel_range_damage(c[1], t[1]);
        const double blue = channel_range_damage(c[2], t[2]);
        const double rg = red > green ? red : green;
        range[i] = rg > blue ? rg : blue;
    }
    Py_END_ALLOW_THREADS
    result = Py_None;
    Py_INCREF(result);

done:
    PyBuffer_Release(&output_view);
    PyBuffer_Release(&t_view);
    PyBuffer_Release(&c_view);
    return result;
}

static PyMethodDef methods[] = {
    {"tile_integral", tile_integral, METH_VARARGS, NULL},
    {"rgb_square_difference", rgb_square_difference, METH_VARARGS, NULL},
    {"rgb_means", rgb_means, METH_VARARGS, NULL},
    {"contrast_damage", contrast_damage, METH_VARARGS, NULL},
    {"range_damage", range_damage, METH_VARARGS, NULL},
    {"lab_xyz", lab_xyz, METH_VARARGS, NULL},
    {"lab_planes", lab_planes, METH_VARARGS, NULL},
    {NULL, NULL, 0, NULL},
};

static struct PyModuleDef module = {
    PyModuleDef_HEAD_INIT,
    "_scoring_native",
    NULL,
    -1,
    methods,
};

PyMODINIT_FUNC
PyInit__scoring_native(void)
{
#if FLT_RADIX != 2 || FLT_MANT_DIG != 24 || FLT_MIN_EXP != -125 || FLT_MAX_EXP != 128 || \
    DBL_MANT_DIG != 53 || DBL_MIN_EXP != -1021 || DBL_MAX_EXP != 1024 || FLT_EVAL_METHOD != 0
    PyErr_SetString(PyExc_ImportError, "The native scorer requires standard IEEE binary32 and binary64 arithmetic");
    return NULL;
#else
    return PyModule_Create(&module);
#endif
}
