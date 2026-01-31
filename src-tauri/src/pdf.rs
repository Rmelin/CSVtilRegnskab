use std::io::BufWriter;
use std::path::PathBuf;

use chrono::Local;
use printpdf::{BuiltinFont, IndirectFontRef, Line, Mm, PdfDocument, Point};
use rust_decimal::Decimal;
use std::str::FromStr;
use sqlx::SqlitePool;

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::SettingsPayload;

const PAGE_WIDTH_MM: f32 = 210.0;
const LEFT_MARGIN_MM: f32 = 15.0;
const RIGHT_MARGIN_MM: f32 = 15.0;
const TABLE_BOTTOM_MM: f32 = 28.0;
const CHAR_WIDTH_FACTOR: f32 = 0.6;
const TABLE_LINE_THICKNESS: f32 = 0.2;
const PT_TO_MM: f32 = 0.352_777_78;
const TABLE_LINE_OFFSET_Y: f32 = 0.6;

pub async fn generate_pdf(pool: &SqlitePool, year: i32) -> AppResult<PathBuf> {
    let (start_balance, end_balance, movements) = db::get_balance_summary(pool, year).await?;

    let settings = db::get_settings_for_year(pool, year).await?;
    let preview = db::get_report_preview(pool, year).await?;
    let notes = db::list_notes(pool, year).await?;
    let balance_curve = db::get_balance_curve(pool, year).await?;
    let title = format!("Foreningsregnskab {}", year);
    let (doc, page1, layer1) = PdfDocument::new(&title, Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|err| AppError::Pdf(format!("Font error: {}", err)))?;
    let table_font = doc
        .add_builtin_font(BuiltinFont::Courier)
        .map_err(|err| AppError::Pdf(format!("Font error: {}", err)))?;
    let generated_at = generation_date();
    let total_income = parse_decimal(&preview.total_income)?;
    let total_expense = parse_decimal(&preview.total_expense)?;
    let result_total = parse_decimal(&preview.result)?;

    let layer = doc.get_page(page1).get_layer(layer1);
    let mut cursor_y: f32 = 280.0;

    if let Some(line) = settings.pdf_title_line1.as_deref() {
        if !line.trim().is_empty() {
            add_line(&layer, &mut cursor_y, &font, line, 16.0);
        }
    }
    if let Some(line) = settings.pdf_title_line2.as_deref() {
        if !line.trim().is_empty() {
            add_line(&layer, &mut cursor_y, &font, line, 12.0);
        }
    }
    add_line(
        &layer,
        &mut cursor_y,
        &font,
        &format!("Regnskab for året {}", year),
        12.0,
    );
    cursor_y -= 2.0;
    cursor_y = render_overview_block(
        &layer,
        &font,
        cursor_y,
        total_income,
        total_expense,
        result_total,
    );

    let income_categories = build_group_blocks(&preview.income_groups);
    let expense_categories = build_group_blocks(&preview.expense_groups);
    let counts = TableRowCounts {
        income_categories: income_categories.len(),
        income_posts: income_categories
            .iter()
            .map(|item| item.posts.len() + 1)
            .sum(),
        expense_categories: expense_categories.len(),
        expense_posts: expense_categories
            .iter()
            .map(|item| item.posts.len() + 1)
            .sum(),
        include_result: false,
    };

    cursor_y -= 2.0;
    let table_layout = build_table_layout_with_counts(cursor_y, counts);
    let mut table_lines = Vec::new();
    let table_top_y = cursor_y + table_layout.sizes.header * 0.7;
    table_lines.push(table_top_y);
    render_table_header(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        preview.year,
        &mut table_lines,
    );
    render_section_heading(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        "INDTÆGTER",
        &mut table_lines,
    );
    render_category_table(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        &income_categories,
        &mut table_lines,
    )?;
    render_totals_row(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        "I alt",
        &preview.total_income,
        &preview.budget_current_total_income,
        &preview.budget_next_total_income,
        &mut table_lines,
    )?;

    cursor_y -= table_layout.gaps.after_section;
    render_section_heading(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        "UDGIFTER",
        &mut table_lines,
    );
    render_category_table(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        &expense_categories,
        &mut table_lines,
    )?;
    render_totals_row(
        &layer,
        &mut cursor_y,
        &table_font,
        &table_layout,
        "I alt",
        &preview.total_expense,
        &preview.budget_current_total_expense,
        &preview.budget_next_total_expense,
        &mut table_lines,
    )?;
    draw_table_grid(&layer, &table_layout, &table_lines);

    add_footer(&layer, &font, 1, &generated_at);

    let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 2");
    let layer = doc.get_page(page2).get_layer(layer2);
    let mut cursor_y: f32 = 280.0;

    add_line(&layer, &mut cursor_y, &font, "Kontobevægelser", 15.0);
    cursor_y -= 4.0;
    let graph_top = cursor_y;
    let graph_bottom = 150.0;
    render_balance_graph(&layer, graph_top, graph_bottom, &balance_curve);
    cursor_y = graph_bottom - 8.0;
    render_balance_table(
        &layer,
        &font,
        cursor_y,
        year,
        start_balance,
        movements,
        end_balance,
    );

    add_footer(&layer, &font, 2, &generated_at);

    let (page3, layer3) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 3");
    let layer = doc.get_page(page3).get_layer(layer3);
    let mut cursor_y: f32 = 280.0;

    add_line(&layer, &mut cursor_y, &font, "Noter til regnskabet", 14.0);
    cursor_y -= 4.0;
    for note in notes {
        let heading = format!("NOTE {}", note.note_number);
        render_note_block(&layer, &mut cursor_y, &font, &heading, Some(note.body.as_str()));
    }

    cursor_y -= 6.0;
    if settings.signatures_enabled.unwrap_or(true) {
        render_signature_block(&layer, &font, cursor_y, &settings);
    }

    add_footer(&layer, &font, 3, &generated_at);

    let mut path = default_download_dir().unwrap_or_else(std::env::temp_dir);
    let date_stamp = Local::now().format("%Y%m%d");
    path.push(format!(
        "foreningsregnskab_report_{}_{}.pdf",
        date_stamp, year
    ));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let file = std::fs::File::create(&path)?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|err| AppError::Pdf(format!("Save error: {}", err)))?;

    Ok(path)
}

fn render_overview_block(
    layer: &printpdf::PdfLayerReference,
    font: &IndirectFontRef,
    start_y: f32,
    total_income: Decimal,
    total_expense: Decimal,
    result_total: Decimal,
) -> f32 {
    let label_x = LEFT_MARGIN_MM;
    let value_right = PAGE_WIDTH_MM - RIGHT_MARGIN_MM;
    let mut cursor = start_y;

    draw_text(layer, font, "Overblik", 11.0, label_x, cursor);
    cursor -= 6.0;
    draw_text(layer, font, "Indtægter i alt", 10.0, label_x, cursor);
    draw_text_right(layer, font, &format_kr(total_income), 10.0, value_right, cursor);
    cursor -= 5.5;
    draw_text(layer, font, "Udgifter i alt", 10.0, label_x, cursor);
    draw_text_right(layer, font, &format_kr(total_expense), 10.0, value_right, cursor);
    cursor -= 5.5;
    draw_text(layer, font, "Resultat", 10.0, label_x, cursor);
    draw_text_right(layer, font, &format_kr(result_total), 10.0, value_right, cursor);
    cursor - 6.0
}

fn default_download_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push("Downloads");
        return Some(path);
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let mut path = PathBuf::from(home);
        path.push("Downloads");
        return Some(path);
    }
    None
}

fn add_line(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    text: &str,
    size: f32,
) {
    layer.use_text(text, size, Mm(20.0), Mm((*cursor_y).into()), font);
    *cursor_y -= size + 4.0;
}

fn add_line_tight(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    text: &str,
    size: f32,
) {
    layer.use_text(text, size, Mm(20.0), Mm((*cursor_y).into()), font);
    *cursor_y -= size + 1.8;
}

#[derive(Clone, Copy)]
struct TableColumns {
    name_x: f32,
    actual_right: f32,
    budget_current_right: f32,
    budget_next_right: f32,
}

#[derive(Clone, Copy)]
struct TableFontSizes {
    header: f32,
    header_small: f32,
    section: f32,
    group: f32,
    row: f32,
    subtotal: f32,
    total: f32,
}

#[derive(Clone, Copy)]
struct TableGaps {
    header: f32,
    header_small: f32,
    section: f32,
    group: f32,
    row: f32,
    subtotal: f32,
    total: f32,
    after_group: f32,
    after_section: f32,
    before_result: f32,
}

#[derive(Clone, Copy)]
struct TableLayout {
    columns: TableColumns,
    sizes: TableFontSizes,
    gaps: TableGaps,
}

struct TableRowCounts {
    income_categories: usize,
    income_posts: usize,
    expense_categories: usize,
    expense_posts: usize,
    include_result: bool,
}

struct CategoryBlock {
    label: String,
    posts: Vec<crate::models::ReportPostSummary>,
}

fn build_group_blocks(
    groups: &[crate::models::ReportGroupSummary],
) -> Vec<CategoryBlock> {
    groups
        .iter()
        .map(|group| CategoryBlock {
            label: group.name.clone(),
            posts: group.posts.clone(),
        })
        .collect()
}

fn sum_category_totals(
    posts: &[crate::models::ReportPostSummary],
) -> AppResult<(Decimal, Decimal, Decimal)> {
    let mut actual_total = Decimal::new(0, 2);
    let mut budget_current = Decimal::new(0, 2);
    let mut budget_next = Decimal::new(0, 2);
    for post in posts {
        actual_total += parse_decimal(&post.total)?;
        if let Some(value) = post.budget_current.as_deref() {
            if !value.trim().is_empty() {
                budget_current += parse_decimal(value)?;
            }
        }
        if let Some(value) = post.budget_next.as_deref() {
            if !value.trim().is_empty() {
                budget_next += parse_decimal(value)?;
            }
        }
    }
    Ok((actual_total, budget_current, budget_next))
}

fn build_table_layout_with_counts(start_y: f32, counts: TableRowCounts) -> TableLayout {
    let columns = TableColumns {
        name_x: LEFT_MARGIN_MM,
        actual_right: PAGE_WIDTH_MM - RIGHT_MARGIN_MM - 45.0,
        budget_current_right: PAGE_WIDTH_MM - RIGHT_MARGIN_MM - 20.0,
        budget_next_right: PAGE_WIDTH_MM - RIGHT_MARGIN_MM,
    };

    let base_sizes = TableFontSizes {
        header: 8.5,
        header_small: 7.0,
        section: 11.0,
        group: 9.0,
        row: 8.2,
        subtotal: 8.2,
        total: 9.0,
    };

    let base_gaps = TableGaps {
        header: 1.0,
        header_small: 0.8,
        section: 1.4,
        group: 1.0,
        row: 0.8,
        subtotal: 0.9,
        total: 1.0,
        after_group: 1.4,
        after_section: 1.6,
        before_result: 2.0,
    };

    let available_height = start_y - TABLE_BOTTOM_MM;
    let base_height = estimate_table_height_with_counts(counts, &base_sizes, &base_gaps);
    let scale = if available_height > 0.0 {
        (available_height / base_height).min(1.0)
    } else {
        1.0
    };

    TableLayout {
        columns,
        sizes: scale_sizes(base_sizes, scale),
        gaps: scale_gaps(base_gaps, scale),
    }
}

fn scale_sizes(sizes: TableFontSizes, scale: f32) -> TableFontSizes {
    TableFontSizes {
        header: sizes.header * scale,
        header_small: sizes.header_small * scale,
        section: sizes.section * scale,
        group: sizes.group * scale,
        row: sizes.row * scale,
        subtotal: sizes.subtotal * scale,
        total: sizes.total * scale,
    }
}

fn scale_gaps(gaps: TableGaps, scale: f32) -> TableGaps {
    TableGaps {
        header: gaps.header * scale,
        header_small: gaps.header_small * scale,
        section: gaps.section * scale,
        group: gaps.group * scale,
        row: gaps.row * scale,
        subtotal: gaps.subtotal * scale,
        total: gaps.total * scale,
        after_group: gaps.after_group * scale,
        after_section: gaps.after_section * scale,
        before_result: gaps.before_result * scale,
    }
}

fn estimate_table_height_with_counts(
    counts: TableRowCounts,
    sizes: &TableFontSizes,
    gaps: &TableGaps,
) -> f32 {
    let mut height = 0.0;
    height += line_height(sizes.header, gaps.header);
    height += line_height(sizes.header_small, gaps.header_small);

    height += line_height(sizes.section, gaps.section);
    height += counts.income_categories as f32 * line_height(sizes.group, gaps.group);
    height += counts.income_posts as f32 * line_height(sizes.row, gaps.row);
    if counts.income_categories > 1 {
        height += (counts.income_categories as f32 - 1.0) * gaps.after_group;
    }
    height += line_height(sizes.total, gaps.total);

    height += gaps.after_section;
    height += line_height(sizes.section, gaps.section);
    height += counts.expense_categories as f32 * line_height(sizes.group, gaps.group);
    height += counts.expense_posts as f32 * line_height(sizes.row, gaps.row);
    if counts.expense_categories > 1 {
        height += (counts.expense_categories as f32 - 1.0) * gaps.after_group;
    }
    height += line_height(sizes.total, gaps.total);

    if counts.include_result {
        height += gaps.before_result;
        height += line_height(sizes.total, gaps.total);
    }
    height
}

fn line_height(size: f32, gap: f32) -> f32 {
    size + gap
}

fn render_table_header(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    layout: &TableLayout,
    year: i32,
    lines: &mut Vec<f32>,
) {
    let columns = layout.columns;
    let sizes = layout.sizes;
    let gaps = layout.gaps;
    draw_text_right(
        layer,
        font,
        &format!("Regnskab {}", year),
        sizes.header,
        columns.actual_right,
        *cursor_y,
    );
    draw_text_right(
        layer,
        font,
        &format!("Budget {}", year),
        sizes.header,
        columns.budget_current_right,
        *cursor_y,
    );
    draw_text_right(
        layer,
        font,
        &format!("Budget {}", year + 1),
        sizes.header,
        columns.budget_next_right,
        *cursor_y,
    );
    advance_cursor(cursor_y, sizes.header, gaps.header);

    draw_text_right(layer, font, "kr.", sizes.header_small, columns.actual_right, *cursor_y);
    draw_text_right(
        layer,
        font,
        "kr.",
        sizes.header_small,
        columns.budget_current_right,
        *cursor_y,
    );
    draw_text_right(
        layer,
        font,
        "kr.",
        sizes.header_small,
        columns.budget_next_right,
        *cursor_y,
    );
    advance_cursor(cursor_y, sizes.header_small, gaps.header_small);
    push_line(lines, *cursor_y + gaps.header_small * 0.5);
}

fn render_section_heading(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    layout: &TableLayout,
    label: &str,
    lines: &mut Vec<f32>,
) {
    draw_text(layer, font, label, layout.sizes.section, layout.columns.name_x, *cursor_y);
    advance_cursor(cursor_y, layout.sizes.section, layout.gaps.section);
    push_line(lines, *cursor_y + layout.gaps.section * 0.5);
}

fn render_category_table(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    layout: &TableLayout,
    categories: &[CategoryBlock],
    lines: &mut Vec<f32>,
) -> AppResult<()> {
    for (index, category) in categories.iter().enumerate() {
        let (actual_total, budget_current, budget_next) =
            sum_category_totals(&category.posts)?;
        let label = category.label.clone();
        render_table_row(
            layer,
            cursor_y,
            font,
            layout,
            &label,
            None,
            &actual_total.to_string(),
            &budget_current.to_string(),
            &budget_next.to_string(),
            layout.sizes.subtotal,
            layout.gaps.subtotal,
            lines,
        )?;

        for post in &category.posts {
            render_table_row(
                layer,
                cursor_y,
                font,
                layout,
                &post.name,
                post.note_number,
                &post.total,
                post.budget_current.as_deref().unwrap_or(""),
                post.budget_next.as_deref().unwrap_or(""),
                layout.sizes.row,
                layout.gaps.row,
                lines,
            )?;
        }

        if index + 1 < categories.len() {
            *cursor_y -= layout.gaps.after_group;
        }
    }
    Ok(())
}

fn render_totals_row(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    layout: &TableLayout,
    label: &str,
    actual: &str,
    budget_current: &str,
    budget_next: &str,
    lines: &mut Vec<f32>,
) -> AppResult<()> {
    render_table_row(
        layer,
        cursor_y,
        font,
        layout,
        label,
        None,
        actual,
        budget_current,
        budget_next,
        layout.sizes.total,
        layout.gaps.total,
        lines,
    )?;
    Ok(())
}

fn format_kr(value: Decimal) -> String {
    format!("{} Kr.", format_danish_decimal(value))
}

fn format_amount(value: &str) -> AppResult<String> {
    if value.trim().is_empty() {
        return Ok("".to_string());
    }
    let decimal = parse_decimal(value)?;
    Ok(format_danish_decimal(decimal))
}

fn format_danish_decimal(value: Decimal) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let value = value.abs().round_dp(2);
    let raw = value.to_string();
    let mut parts = raw.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("00");
    let mut frac = frac_part.to_string();
    if frac.len() == 1 {
        frac.push('0');
    } else if frac.len() > 2 {
        frac.truncate(2);
    } else if frac.is_empty() {
        frac = "00".to_string();
    }

    let mut int_with_sep = String::new();
    for (index, ch) in int_part.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            int_with_sep.push('.');
        }
        int_with_sep.push(ch);
    }
    let int_with_sep: String = int_with_sep.chars().rev().collect();
    format!("{}{},{}", sign, int_with_sep, frac)
}

fn parse_decimal(value: &str) -> AppResult<Decimal> {
    Decimal::from_str(value).map_err(|_| AppError::Parse("Invalid decimal".to_string()))
}

fn render_table_row(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    layout: &TableLayout,
    name: &str,
    note_number: Option<i64>,
    actual: &str,
    budget_current: &str,
    budget_next: &str,
    size: f32,
    gap: f32,
    lines: &mut Vec<f32>,
) -> AppResult<()> {
    let actual = format_amount(actual)?;
    let budget_current = format_amount(budget_current)?;
    let budget_next = format_amount(budget_next)?;
    let name_text = truncate_text(
        name,
        layout.columns.actual_right - layout.columns.name_x - 4.0,
        size,
    );
    draw_text(
        layer,
        font,
        &name_text,
        size,
        layout.columns.name_x,
        *cursor_y,
    );
    let note_text = note_number.map(|value| format!("({})", value)).unwrap_or_default();
    let actual_text = format!("{}{}", actual, note_text);
    draw_text_right(
        layer,
        font,
        &actual_text,
        size,
        layout.columns.actual_right,
        *cursor_y,
    );
    draw_text_right(
        layer,
        font,
        &budget_current,
        size,
        layout.columns.budget_current_right,
        *cursor_y,
    );
    draw_text_right(
        layer,
        font,
        &budget_next,
        size,
        layout.columns.budget_next_right,
        *cursor_y,
    );
    advance_cursor(cursor_y, size, gap);
    push_line(lines, *cursor_y + gap * 0.5);
    Ok(())
}

fn generation_date() -> String {
    chrono::Local::now().format("%d.%m.%Y").to_string()
}

fn add_footer(layer: &printpdf::PdfLayerReference, font: &IndirectFontRef, page: i32, date: &str) {
    let text = format!("Side {} af 3 · {}", page, date);
    layer.use_text(text, 8.0, Mm(20.0), Mm(15.0), font);
}

fn draw_text(
    layer: &printpdf::PdfLayerReference,
    font: &IndirectFontRef,
    text: &str,
    size: f32,
    x: f32,
    y: f32,
) {
    layer.use_text(text, size, Mm(x), Mm(y), font);
}

fn draw_text_right(
    layer: &printpdf::PdfLayerReference,
    font: &IndirectFontRef,
    text: &str,
    size: f32,
    right_x: f32,
    y: f32,
) {
    let width = estimate_text_width(text, size);
    let x = (right_x - width).max(LEFT_MARGIN_MM);
    layer.use_text(text, size, Mm(x), Mm(y), font);
}

fn estimate_text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * CHAR_WIDTH_FACTOR * PT_TO_MM
}

fn advance_cursor(cursor_y: &mut f32, size: f32, gap: f32) {
    *cursor_y -= size + gap;
}

fn truncate_text(text: &str, max_width_mm: f32, size: f32) -> String {
    if estimate_text_width(text, size) <= max_width_mm {
        return text.to_string();
    }
    let mut truncated = String::new();
    let mut width = 0.0;
    let ellipsis = "...";
    let ellipsis_width = estimate_text_width(ellipsis, size);
    for ch in text.chars() {
        let ch_width = estimate_text_width(&ch.to_string(), size);
        if width + ch_width + ellipsis_width > max_width_mm {
            break;
        }
        truncated.push(ch);
        width += ch_width;
    }
    if truncated.is_empty() {
        ellipsis.to_string()
    } else {
        truncated.push_str(ellipsis);
        truncated
    }
}

fn draw_table_grid(layer: &printpdf::PdfLayerReference, layout: &TableLayout, lines: &[f32]) {
    if lines.is_empty() {
        return;
    }
    let mut sorted_lines = lines.to_vec();
    sorted_lines.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let table_right = PAGE_WIDTH_MM - RIGHT_MARGIN_MM;
    let top_y = sorted_lines.first().copied().unwrap_or(0.0);
    let bottom_y = sorted_lines.last().copied().unwrap_or(0.0);

    for y in sorted_lines {
        let line_y = y - TABLE_LINE_OFFSET_Y;
        draw_line(layer, LEFT_MARGIN_MM, line_y, table_right, line_y, TABLE_LINE_THICKNESS);
    }

    let verticals = [
        LEFT_MARGIN_MM,
        layout.columns.actual_right + 2.0,
        layout.columns.budget_current_right + 2.0,
        table_right,
    ];
    for x in verticals {
        draw_line(layer, x, top_y, x, bottom_y, TABLE_LINE_THICKNESS);
    }
}

fn draw_line(
    layer: &printpdf::PdfLayerReference,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    thickness: f32,
) {
    layer.set_outline_thickness(thickness);
    let line = Line {
        points: vec![
            (Point::new(Mm(x1), Mm(y1)), false),
            (Point::new(Mm(x2), Mm(y2)), false),
        ],
        is_closed: false,
    };
    layer.add_line(line);
}

fn render_balance_graph(
    layer: &printpdf::PdfLayerReference,
    top_y: f32,
    bottom_y: f32,
    points: &[crate::models::BalancePoint],
) {
    if points.is_empty() {
        return;
    }
    let left_x = LEFT_MARGIN_MM;
    let right_x = PAGE_WIDTH_MM - RIGHT_MARGIN_MM;
    let width = right_x - left_x;
    let height = top_y - bottom_y;

    let mut min_value = f64::MAX;
    let mut max_value = f64::MIN;
    for point in points {
        if point.balance < min_value {
            min_value = point.balance;
        }
        if point.balance > max_value {
            max_value = point.balance;
        }
    }
    if (max_value - min_value).abs() < f64::EPSILON {
        max_value += 1.0;
        min_value -= 1.0;
    }

    let padding = (max_value - min_value) * 0.05;
    let max_value = max_value + padding;
    let min_value = min_value - padding;
    let range = max_value - min_value;

    draw_line(layer, left_x, bottom_y, right_x, bottom_y, 0.3);
    draw_line(layer, left_x, bottom_y, left_x, top_y, 0.3);

    let count = points.len();
    let mut prev_x = left_x;
    let mut prev_y = bottom_y;
    for (index, point) in points.iter().enumerate() {
        let x = left_x + (index as f32 / (count.saturating_sub(1) as f32).max(1.0)) * width;
        let ratio = ((point.balance - min_value) / range) as f32;
        let y = bottom_y + ratio * height;
        if index > 0 {
            draw_line(layer, prev_x, prev_y, x, y, 0.4);
        }
        prev_x = x;
        prev_y = y;
    }
}

fn render_balance_table(
    layer: &printpdf::PdfLayerReference,
    font: &IndirectFontRef,
    start_y: f32,
    year: i32,
    start_balance: Decimal,
    movements: Decimal,
    end_balance: Decimal,
) -> f32 {
    let left_label_x = LEFT_MARGIN_MM;
    let left_value_right = 95.0;
    let right_label_x = 115.0;
    let right_value_right = PAGE_WIDTH_MM - RIGHT_MARGIN_MM;

    let mut cursor = start_y;
    draw_text(
        layer,
        font,
        &format!("BALANCE PR. 31.12.{}", year),
        11.0,
        left_label_x,
        cursor,
    );
    cursor -= 6.0;
    draw_text(layer, font, "AKTIVER", 10.0, left_label_x, cursor);
    draw_text(layer, font, "PASSIVER", 10.0, right_label_x, cursor);
    cursor -= 5.5;

    draw_text(layer, font, "Bankbeholdning", 9.5, left_label_x, cursor);
    draw_text(layer, font, "Egenkapital", 9.5, right_label_x, cursor);
    cursor -= 5.0;
    draw_text(layer, font, "Primo", 9.0, left_label_x + 20.0, cursor);
    draw_text_right(layer, font, &format_kr(start_balance), 9.0, left_value_right, cursor);
    draw_text(layer, font, "Primo", 9.0, right_label_x + 20.0, cursor);
    draw_text_right(layer, font, &format_kr(start_balance), 9.0, right_value_right, cursor);
    cursor -= 4.5;
    draw_text(layer, font, "Bevægelser", 9.0, left_label_x + 20.0, cursor);
    draw_text_right(layer, font, &format_kr(movements), 9.0, left_value_right, cursor);
    draw_text(layer, font, "Bevægelser", 9.0, right_label_x + 20.0, cursor);
    draw_text_right(layer, font, &format_kr(movements), 9.0, right_value_right, cursor);
    cursor -= 4.5;
    draw_text(layer, font, "Ultimo", 9.0, left_label_x + 20.0, cursor);
    draw_text_right(layer, font, &format_kr(end_balance), 9.0, left_value_right, cursor);
    draw_text(layer, font, "Ultimo", 9.0, right_label_x + 20.0, cursor);
    draw_text_right(layer, font, &format_kr(end_balance), 9.0, right_value_right, cursor);
    cursor -= 5.5;
    draw_text(layer, font, "Aktiver i alt", 9.5, left_label_x, cursor);
    draw_text_right(
        layer,
        font,
        &format_kr(end_balance),
        9.5,
        left_value_right,
        cursor,
    );
    draw_text(layer, font, "Passiver i alt", 9.5, right_label_x, cursor);
    draw_text_right(
        layer,
        font,
        &format_kr(end_balance),
        9.5,
        right_value_right,
        cursor,
    );
    cursor - 6.0
}

fn render_signature_block(
    layer: &printpdf::PdfLayerReference,
    font: &IndirectFontRef,
    start_y: f32,
    settings: &SettingsPayload,
) -> f32 {
    let mut cursor = start_y;
    draw_text(
        layer,
        font,
        "Regnskabet er gennemgået af revisor. Bankkontoen stemmer med regnskabet, og der er ingen bemærkninger.",
        9.5,
        LEFT_MARGIN_MM,
        cursor,
    );
    cursor -= 8.0;

    let col1 = LEFT_MARGIN_MM;
    let col2 = 90.0;
    let col3 = 160.0;
    let line = "____________________";

    draw_text(layer, font, "Formand", 9.0, col1, cursor);
    draw_text(layer, font, settings.chair.as_deref().unwrap_or(""), 9.0, col1, cursor - 4.0);
    draw_text(layer, font, line, 9.0, col1, cursor - 8.0);

    draw_text(layer, font, "Bestyrelsesmedlem", 9.0, col2, cursor);
    draw_text(
        layer,
        font,
        settings.board_member_one.as_deref().unwrap_or(""),
        9.0,
        col2,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col2, cursor - 8.0);

    draw_text(layer, font, "Kasser", 9.0, col3, cursor);
    draw_text(
        layer,
        font,
        settings.treasurer.as_deref().unwrap_or(""),
        9.0,
        col3,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col3, cursor - 8.0);

    cursor -= 16.0;

    draw_text(layer, font, "Bestyrelsesmedlem", 9.0, col1, cursor);
    draw_text(
        layer,
        font,
        settings.board_member_two.as_deref().unwrap_or(""),
        9.0,
        col1,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col1, cursor - 8.0);

    draw_text(layer, font, "Bestyrelsesmedlem", 9.0, col2, cursor);
    draw_text(
        layer,
        font,
        settings.board_member_three.as_deref().unwrap_or(""),
        9.0,
        col2,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col2, cursor - 8.0);

    draw_text(layer, font, "Bestyrelsesmedlem", 9.0, col3, cursor);
    draw_text(
        layer,
        font,
        settings.board_member_four.as_deref().unwrap_or(""),
        9.0,
        col3,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col3, cursor - 8.0);

    cursor -= 16.0;

    draw_text(layer, font, "Revisor", 9.0, col1, cursor);
    draw_text(
        layer,
        font,
        settings.auditor_one.as_deref().unwrap_or(""),
        9.0,
        col1,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col1, cursor - 8.0);

    draw_text(layer, font, "Revisor", 9.0, col2, cursor);
    draw_text(
        layer,
        font,
        settings.auditor_two.as_deref().unwrap_or(""),
        9.0,
        col2,
        cursor - 4.0,
    );
    draw_text(layer, font, line, 9.0, col2, cursor - 8.0);

    cursor - 10.0
}

fn push_line(lines: &mut Vec<f32>, y: f32) {
    lines.push(y);
}

fn render_note_block(
    layer: &printpdf::PdfLayerReference,
    cursor_y: &mut f32,
    font: &IndirectFontRef,
    label: &str,
    text: Option<&str>,
) {
    add_line(layer, cursor_y, font, label, 10.0);
    if let Some(content) = text {
        let lines = wrap_text(content, 80);
        for line in lines {
            add_line_tight(layer, cursor_y, font, &line, 9.0);
        }
    }
    *cursor_y -= 4.0;
}

fn wrap_text(text: &str, max_len: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > max_len {
            if !current.is_empty() {
                lines.push(current.trim_end().to_string());
            }
            current = String::new();
        }
        current.push_str(word);
        current.push(' ');
    }
    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }
    if lines.is_empty() {
        lines.push("".to_string());
    }
    lines
}
