import { useEffect, useMemo, useState } from "react";
import DataTable from "../components/table/DataTable";
import Badge from "../components/status/Badge";
import RadarChart from "../components/charts/RadarChart";
import Modal from "../components/dialog/Modal";
import { categoryLabelMap, categoryOrder } from "../lib/categories";
import { useAnalysis } from "../features/analysis/store";
import { PhotoDetail, PhotoRow } from "../lib/api/types";
import { getPhotoDetail } from "../lib/api";

function ResultsPage() {
  const {
    photos,
    distributionAvg,
    distributionCount,
    valueStats,
    settingsValueEnabled,
    categoryFilter,
    setCategoryFilter,
    loadPhotoDetail,
    selectedPhoto,
    closeDetail,
    loadingDetail,
  } = useAnalysis();

  const filteredPhotos = useMemo(() => {
    if (categoryFilter === "all") return photos;
    return photos.filter((p) => p.category === categoryFilter);
  }, [photos, categoryFilter]);

  const [rowLimit, setRowLimit] = useState<number>(10);
  const [page, setPage] = useState<number>(1);
  const [pageInput, setPageInput] = useState<string>("1");

  const [logOpen, setLogOpen] = useState(false);
  const [logLoading, setLogLoading] = useState(false);
  const [logDetail, setLogDetail] = useState<PhotoDetail | null>(null);

  const pageCount = Math.max(1, Math.ceil((filteredPhotos.length || 1) / (rowLimit || 1)));
  const pageStart = (page - 1) * rowLimit;
  const limitedPhotos = useMemo(
    () => filteredPhotos.slice(pageStart, pageStart + rowLimit),
    [filteredPhotos, pageStart, rowLimit]
  );

  useEffect(() => {
    if (page > pageCount) {
      setPage(pageCount);
      setPageInput(String(pageCount));
    }
  }, [page, pageCount]);

  useEffect(() => {
    setPage(1);
    setPageInput("1");
  }, [categoryFilter, rowLimit, filteredPhotos.length]);

  useEffect(() => {
    setPageInput(String(page));
  }, [page]);

  const openLogFor = async (row: PhotoRow) => {
    setLogOpen(true);
    setLogLoading(true);
    try {
      const detail = await getPhotoDetail(row.id);
      setLogDetail(detail);
    } catch {
      setLogDetail({
        ...row,
        caption: undefined,
        textInImage: undefined,
        analysisLog: row.errorMessage ?? "로그를 불러오지 못했습니다.",
      });
    } finally {
      setLogLoading(false);
    }
  };

  const columns = [
    {
      key: "fileName",
      header: "파일명",
      render: (r: PhotoRow) => (
        <button
          className="link-btn"
          onClick={(e) => {
            e.stopPropagation();
            openLogFor(r);
          }}
        >
          {r.fileName}
        </button>
      ),
    },
    {
      key: "analysisDurationMs",
      header: "시간",
      render: (r: PhotoRow) =>
        typeof r.analysisDurationMs === "number"
          ? `${(r.analysisDurationMs / 1000).toFixed(1)}s`
          : "-",
    },
    { key: "path", header: "경로" },
    {
      key: "category",
      header: "카테고리",
      render: (r: PhotoRow) => categoryLabelMap[r.category],
    },
    {
      key: "topScore",
      header: "Top score",
      render: (r: PhotoRow) => `${(r.topScore * 100).toFixed(1)}%`,
    },
    {
      key: "exportStatus",
      header: "Export",
      render: (r: PhotoRow) =>
        r.exportStatus === "success" ? (
          <Badge label="success" tone="success" />
        ) : r.exportStatus === "error" ? (
          <Badge label="error" tone="danger" />
        ) : (
          <Badge label="pending" tone="neutral" />
        ),
    },
    {
      key: "errorMessage",
      header: "에러",
      render: (r: PhotoRow) => r.errorMessage ?? "-",
    },
  ];

  const [detailLoadingId, setDetailLoadingId] = useState<string | null>(null);

  const handleRowClick = async (row: PhotoRow) => {
    setDetailLoadingId(row.id);
    await loadPhotoDetail(row.id);
    setDetailLoadingId(null);
  };

  return (
    <div className="page">
      <h1>결과</h1>
      <p className="muted">카테고리 필터, 테이블, 분포를 확인하세요.</p>

      <div className="section card">
        <div className="flex-between">
          <div className="section-title">카테고리 필터</div>
          <div className="muted">8개 축 고정</div>
        </div>
        <div className="chips">
          <button
            className={`chip ${categoryFilter === "all" ? "active" : ""}`}
            onClick={() => setCategoryFilter("all")}
          >
            전체
          </button>
          {categoryOrder.map((c) => (
            <button
              key={c}
              className={`chip ${categoryFilter === c ? "active" : ""}`}
              onClick={() => setCategoryFilter(c)}
            >
              {categoryLabelMap[c]}
            </button>
          ))}
        </div>
      </div>

      <div className="section card">
        <div className="flex-between" style={{ marginBottom: 10 }}>
          <div className="section-title">결과 테이블</div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <label className="muted" htmlFor="row-limit">
              표시 개수
            </label>
            <select
              id="row-limit"
              className="select"
              value={rowLimit}
              onChange={(e) => {
                setRowLimit(Number(e.target.value));
                setPage(1);
                setPageInput("1");
              }}
            >
              {[10, 25, 50, 100].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
              <option value={filteredPhotos.length}>전체</option>
            </select>
          </div>
        </div>
        <DataTable
          columns={columns}
          data={limitedPhotos}
          onRowClick={handleRowClick}
          emptyText="결과가 없습니다. 분석을 실행하세요."
        />
        <div className="section" style={{ marginTop: 14 }}>
          <button
            className="pager-btn"
            onClick={() => setLogOpen((v) => !v)}
            disabled={!logDetail && !logLoading}
          >
            {logOpen ? "로그 접기" : "로그 보기"}
          </button>
          {logOpen && (
            <div className="card log-panel" style={{ marginTop: 10 }}>
              <div className="flex-between">
                <div className="section-title">작업 로그</div>
                {logLoading && <span className="muted">불러오는 중…</span>}
              </div>
              {logDetail ? (
                <>
                  <div className="muted" style={{ marginTop: 8 }}>
                    파일: {logDetail.fileName} · 상태: {logDetail.exportStatus}
                  </div>
                  {logDetail.errorMessage && (
                    <div className="muted" style={{ marginTop: 8 }}>
                      에러: {logDetail.errorMessage}
                    </div>
                  )}
                  <pre className="log-pre">{logDetail.analysisLog ?? "(로그 없음)"}</pre>
                </>
              ) : (
                <div className="muted" style={{ marginTop: 10 }}>
                  파일명을 클릭하면 해당 작업 로그가 표시됩니다.
                </div>
              )}
            </div>
          )}
        </div>
        <div className="pagination">
          <button
            className="pager-btn"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={page <= 1}
          >
            이전
          </button>
          <div className="pager-input-wrap">
            <input
              className="pager-input"
              inputMode="numeric"
              value={pageInput}
              onChange={(e) => setPageInput(e.target.value)}
              onBlur={() => {
                const n = Number(pageInput);
                if (!Number.isFinite(n)) {
                  setPageInput(String(page));
                  return;
                }
                const clamped = Math.min(Math.max(1, n), pageCount);
                setPage(clamped);
                setPageInput(String(clamped));
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  const n = Number(pageInput);
                  if (!Number.isFinite(n)) {
                    setPageInput(String(page));
                    return;
                  }
                  const clamped = Math.min(Math.max(1, n), pageCount);
                  setPage(clamped);
                  setPageInput(String(clamped));
                }
              }}
            />
            <span className="muted"> / {pageCount}</span>
          </div>
          <span className="muted">(총 {filteredPhotos.length}개)</span>
          <button
            className="pager-btn"
            onClick={() => setPage((p) => Math.min(pageCount, p + 1))}
            disabled={page >= pageCount}
          >
            다음
          </button>
        </div>
      </div>

      <div className="section card">
        <div className="section-title">전체 분포 레이더</div>
        {distributionAvg || distributionCount ? (
          <>
            <div className="radar-grid">
              {distributionAvg && (
                <div className="radar-wrap">
                  <RadarChart scores={distributionAvg.byCategory} title="평균 점수" />
                </div>
              )}
              {distributionCount && (
                <div className="radar-wrap">
                  <RadarChart scores={distributionCount.byCategory} title="개수 비율" />
                </div>
              )}
            </div>
            {settingsValueEnabled && valueStats && (
              <div className="section" style={{ marginTop: 14 }}>
                <div className="section-title">저장 가치(1단계) 분포</div>
                {valueStats.valuable + valueStats.notValuable > 0 ? (
                  (() => {
                    const total = valueStats.valuable + valueStats.notValuable;
                    const keepPct = (valueStats.valuable / total) * 100;
                    const dropPct = (valueStats.notValuable / total) * 100;
                    return (
                      <>
                        <div className="value-bar" aria-label="저장 가치 분포">
                          <div
                            className="value-seg keep"
                            style={{ width: `${keepPct}%` }}
                            title={`가치 있음 ${valueStats.valuable}장`}
                          />
                          <div
                            className="value-seg drop"
                            style={{ width: `${dropPct}%` }}
                            title={`가치 없음 ${valueStats.notValuable}장`}
                          />
                        </div>
                        <div className="flex-between" style={{ marginTop: 8 }}>
                          <div className="muted">
                            가치 있음: <b>{valueStats.valuable}</b>장
                          </div>
                          <div className="muted">
                            가치 없음: <b>{valueStats.notValuable}</b>장
                          </div>
                        </div>
                      </>
                    );
                  })()
                ) : (
                  <p className="muted" style={{ marginTop: 8 }}>
                    아직 가치 판단 결과가 없습니다.
                  </p>
                )}
              </div>
            )}
            <div className="dist-table">
              <table>
                <thead>
                  <tr>
                    <th>카테고리</th>
                    <th>평균 점수</th>
                    <th>개수 비율</th>
                  </tr>
                </thead>
                <tbody>
                  {categoryOrder.map((c) => (
                    <tr key={c}>
                      <td>{categoryLabelMap[c]}</td>
                      <td>
                        {distributionAvg
                          ? `${(distributionAvg.byCategory[c] * 100).toFixed(1)}%`
                          : "-"}
                      </td>
                      <td>
                        {distributionCount
                          ? `${(distributionCount.byCategory[c] * 100).toFixed(1)}%`
                          : "-"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        ) : (
          <p className="muted">분석 완료 후 표시됩니다.</p>
        )}
      </div>

      <Modal
        open={Boolean(selectedPhoto)}
        onClose={closeDetail}
        title={selectedPhoto ? selectedPhoto.fileName : "상세"}
      >
        {loadingDetail || detailLoadingId ? (
          <p className="muted">불러오는 중...</p>
        ) : selectedPhoto ? (
          <div className="grid">
            <div>
              <div className="muted">경로</div>
              <div>{selectedPhoto.path}</div>
            </div>
            <div>
              <div className="muted">카테고리</div>
              <div>{categoryLabelMap[selectedPhoto.category]}</div>
            </div>
            <div className="radar-wrap">
              <RadarChart scores={selectedPhoto.scores} />
            </div>
          </div>
        ) : (
          <p className="muted">행을 선택하세요.</p>
        )}
      </Modal>
    </div>
  );
}

export default ResultsPage;
